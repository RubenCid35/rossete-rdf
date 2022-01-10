
use lazy_static::lazy_static;

use crate::mappings::{
    parts::Parts,
    maps::Mapping,
    AcceptedType
};

use crate::ResultApp;
use crate::errors::ApplicationErrors;
use crate::{warning, info, error};

use std::path;
use std::fs;
use std::io::prelude::{Read};
use std::sync::{RwLock, Arc, mpsc};
use std::collections::HashMap;
use regex::Regex;

/// Main Function that is used to create mapping from a RML File in tbe TTL Format.
pub fn parse_text(id: i32, file: path::PathBuf, transmitter: mpsc::Sender<ResultApp<Vec<Mapping>>>, prefix_transmiter: Arc<RwLock<HashMap<String, String>>>, status_transmitter: mpsc::Sender<i32>, debug: bool) -> ResultApp<()>{
    info!("Parsing File ID: {:2.} PATH: {}",  id, file.display());
    // File Reading
    let mut map_file = match fs::File::open(file){
        Ok(file) => file,
        Err(error) => {
            status_transmitter.send(- id)?;
            return Err(error.into())
        }
    };
    let meta = map_file.metadata().unwrap(); // Para sacar los metadatos y prelocate memory for the buffer.
    let mut buffer = String::with_capacity(meta.len() as usize);
    match map_file.read_to_string(&mut buffer){
        Ok(_) => {},
        Err(error) => {
            status_transmitter.send(2)?;
            return Err(error.into());
        }
    }

    // Tokenize the file so we can be parsed.
    let tokens = tokenize(buffer);

    // Get the tokens into diferent mappings and prefixes.
    match parse_tokens(tokens, prefix_transmiter, debug){
        Ok(mappings) => {
            transmitter.send(Ok(mappings))?;
            status_transmitter.send(id)?;
            return Ok(())    
        }
        Err(error) => {
            transmitter.send(Err(error.clone()))?;
            status_transmitter.send(id)?;
            return Err(error)
        }
    }

}

/// Divide the file into words or tokens so they can be processed quickier.
fn tokenize(text: String) -> Vec<String>{
    text
    .split('\n')
    .flat_map(|sentence| sentence.split(' ').map(|word| word.trim()))
    .map(|token| token.replace(';', "").replace('"', ""))
    .filter(|word| !word.is_empty())
    .collect()
}

/// Get the index of the closing bracket if there is.
fn find_closing_bracket(map_str: &Vec<String>, initial: usize) -> Option<usize>{
    let mut close = initial;
    let mut closing = 0;
    loop{
        if map_str.len() <= close{
            return None
        }
        if map_str[close].len() > 2{
            close += 1;
            continue
        }
        if map_str[close].contains(']'){
            if closing == 1{
                return Some(close);
            }else{
                closing -= 1;
            }
        }else if map_str[close].contains('['){
            closing += 1;
        }

        close += 1;
    }
}

/// Parse the main 5 Mapping Parts and create all the mappings and prefixes.
fn parse_tokens(tokens: Vec<String>, prefix_transmiter: Arc<RwLock<HashMap<String, String>>>, debug: bool) -> ResultApp<Vec<Mapping>>{

    let mut mappings: Vec<Mapping> = Vec::with_capacity(2);
    // let mut prefixes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut base_uri = String::new();
    let mut idx = 0;
    while idx < tokens.len(){
        lazy_static!{
            static ref PREFIX: Regex = Regex::new(r#"@(prefix|PREFIX)"#).unwrap();
            static ref PREFIX_URL: Regex = Regex::new(r#"<(https?://[a-zA-Z0-9:\.\#/]{0,256})>\."#).unwrap();
            static ref BASE: Regex = Regex::new(r#"@(base|BASE)"#).unwrap();
            
            static ref MAPPING_INIT: Regex = Regex::new(r#"<#([a-zA-Z0-9_\-]*)>"#).unwrap();
            
            static ref LOGICALSOURCE: Regex = Regex::new(r#"(rr)?:logicalSource"#).unwrap();
            static ref SUBJECTMAP: Regex = Regex::new(r#"(rr)?:subjectMap"#).unwrap();
            static ref PREDICATEOBJECTMAP: Regex = Regex::new(r#"(rr)?:predicateObjectMap"#).unwrap();
        }
        if PREFIX.is_match(&tokens[idx]) && (idx + 2) < tokens.len(){
            let pre = tokens[idx + 1].clone();
            let url = {
                match PREFIX_URL.captures(&tokens[idx + 2]){
                    Some(cap) => {
                        cap.get(1).unwrap().as_str().to_string()
                    }
                    None => {
                        error!("Error While Formatting the URL in the PREFIXES");
                        return Err(ApplicationErrors::IncorrectMappingFormat);    
                    }
                }
            };
            // Se manda a una zona central.
            let mut prefix_zone  = prefix_transmiter.write()?;
            prefix_zone.insert(pre, url);
            drop(prefix_zone);

            // prefixes.insert(pre, url);
            idx += 2;
        }else if BASE.is_match(&tokens[idx]) && (idx + 1) < tokens.len(){
            let url = {
                match PREFIX_URL.captures(&tokens[idx + 1]){
                    Some(cap) => {
                        cap.get(1).unwrap().as_str().to_string()
                    }
                    None => {
                        error!("Error While Formatting the URL in the PREFIXES");
                        return Err(ApplicationErrors::IncorrectMappingFormat);    
                    }
                }
            };
            base_uri = url;
            idx += 1;
        }else if let Some(cap) = MAPPING_INIT.captures(&tokens[idx]){
            let name = cap.get(1).unwrap().as_str().to_string();
            if debug{
                info!("The following mapping was found {}. Parsing Has Started", &name);
            }
            let map = Mapping::new(name);

            mappings.push(map);
        }else if LOGICALSOURCE.is_match(&tokens[idx]){
            if !&tokens[idx + 1].contains('['){
                let map_name = &mappings.last().unwrap().identificador;
                error!("The Mapping {} requires at least a rml:source component", map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat)
            }
            else if let Some(finish) = find_closing_bracket(&tokens, idx + 1){
                let logicalsource = parse_logical_source(&tokens, idx + 2, finish)?;
                mappings.last_mut().unwrap().add_component(logicalsource);
                idx = finish;
            }
            else{
                let map_name = &mappings.last().unwrap().identificador;
                error!("In the Mapping {}, the logicalSource requieres ] at some point to finish the statement", map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat);     
            }

        }else if SUBJECTMAP.is_match(&tokens[idx]){
            if !&tokens[idx + 1].contains('['){
                let map_name = &mappings.last().unwrap().identificador;
                error!("The Mapping {} requires at least a rr:template component", map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat)
            }
            else if let Some(finish) = find_closing_bracket(&tokens, idx + 1){
                let subject_map = parse_subject_map(&tokens, idx + 2, finish)?;
                mappings.last_mut().unwrap().add_component(subject_map);
                idx = finish;
            }
            else{
                let map_name = &mappings.last().unwrap().identificador;
                error!("In the Mapping {}, the subjectMap requieres ] at some point to finish the statement", map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat);     
            }

        }else if PREDICATEOBJECTMAP.is_match(&tokens[idx]){
            ////info!("A predicateObjectMap was parsed in the line {}", idx);
            if !&tokens[idx + 1].contains('['){
                let map_name = &mappings.last().unwrap().identificador;
                error!("In the Mapping {}, the rr:predicateObjectMap requires at least a rr:predicate and rr:objectMap component", map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat)
            }
            else if let Some(finish) = find_closing_bracket(&tokens, idx + 1){
                let predicate_map = parse_predicate_map(&tokens, idx + 2, finish)?;
                mappings.last_mut().unwrap().add_component(predicate_map);
                idx = finish;
            }
            else{
                let map_name = &mappings.last().unwrap().identificador;
                error!("In the Mapping {}, the rr:predicateObjectMap requieres ] at some point to finish the statement", map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat);     
            }   
        }else{
            // To get the last map identification
            let last_map = match mappings.last(){
                Some(map) => {
                    &map.identificador
                }
                None => "No Map Was Created"
            };
            warning!("An Identified Element has appeared in the Term Index: {}. Term: {}. Last Mapping: {} Last Token: {}", idx, &tokens[idx], last_map, &tokens[idx - 1]);
        }
        
        idx += 1;
    }
    
    // Check if all the map have the requiered components: logicalSource and SubjectMap.
    for map in mappings.iter_mut(){
        if let Err(error) = map.is_valid(){
            return Err(error)
        }
        // To add the base uri.
        map.base_uri = base_uri.clone();
    }
    Ok(mappings)

}


// --------- Component Parsing ---------------
fn parse_logical_source(tokens: &Vec<String>, init: usize, end: usize) -> ResultApp<Parts>{
    let mut idx = init;
    let mut file_path = String::with_capacity(255);
    let mut iterator = String::new();
    let mut file_type = AcceptedType::Unspecify;

    while idx < end {
        lazy_static!{
            static ref SOURCE: Regex = Regex::new("rml:source").unwrap();
            static ref ITERATOR: Regex = Regex::new("rml:iterator").unwrap();
            static ref IS_FILE_TYPE: Regex = Regex::new("rml:referenceFormulation").unwrap();
            static ref FILE_TYPE: Regex = Regex::new(r#"ql:(\w+)"#).unwrap();
        }
        if SOURCE.is_match(&tokens[idx]){
            file_path = tokens[idx + 1].clone();
            idx += 1;
        }else if ITERATOR.is_match(&tokens[idx]){
            iterator = tokens[idx + 1].clone();
            idx += 1;
        }else if IS_FILE_TYPE.is_match(&tokens[idx]){
            if let Some(cap) = FILE_TYPE.captures(&tokens[idx + 1]){
                file_type = AcceptedType::from_str(&cap.get(1).unwrap().as_str().to_lowercase());
            }
            idx += 1;
        }else{
            warning!("Some unknown tokens has appeared in the logicalSource, TOKEN: {}", &tokens[idx])
        }

        idx += 1;
    }



    Ok(Parts::LogicalSource{
        source: path::PathBuf::from(file_path),
        reference_formulation: file_type,
        iterator
    })
}

fn parse_subject_map(tokens: &Vec<String>, init: usize, end: usize) -> ResultApp<Parts>{
    let mut comps: Vec<Parts> = Vec::with_capacity(2);
    let mut idx = init;
    while idx < end {
        lazy_static!{
            static ref TEMPLATE: Regex = Regex::new("rr:template").unwrap();
            static ref CONSTANT: Regex = Regex::new("rr:constant").unwrap();
            static ref GRAPHMAP: Regex = Regex::new("rr:GraphMap").unwrap();
            static ref CLASSTYPE: Regex = Regex::new("rr:class").unwrap();
            static ref TERMTYPE: Regex = Regex::new("rr:termType").unwrap();
        }
        if TEMPLATE.is_match(&tokens[idx]){
            let (template, input_fields) = parse_input_field(&tokens[idx + 1])?;
            comps.push(Parts::Template{
                template,
                input_fields,
            });
            idx += 1; 
        }
        else if GRAPHMAP.is_match(&tokens[idx]){
            let comp: Parts;
            if CONSTANT.is_match(&tokens[idx + 1]){
               comp = Parts::Constant(tokens[idx + 2].clone());
               idx += 2; 
            }else{
                comp = Parts::Term(tokens[idx + 1].clone());
                idx += 1;
            }

            let graph = Parts::GraphMap(Box::new(comp));
            comps.push(graph);
        }
        else if CLASSTYPE.is_match(&tokens[idx]){
            comps.push(Parts::Class(tokens[idx + 1].clone()));
            idx += 1;
        }

        idx += 1;
    }

    Ok(Parts::SubjectMap{
        components: comps,
    })
}

fn parse_input_field(elem_uri: &str) -> ResultApp<(String, Vec<String>)>{
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut add = false;
    let mut modified_template = String::with_capacity(elem_uri.len());
    for c in elem_uri.clone().chars(){
        if c == '\"' || c == ';'{
            continue
        }else if c == '{'{
            add = true;
            modified_template.push(c);
        }else if c == '}'{
            add = false;

            fields.push(current_field.clone());
            current_field.clear();

            modified_template.push(c);
        }else{
            if add{
                current_field.push(c);
            }else{
                modified_template.push(c);
            }
        }
        
    }
    if fields.is_empty(){
        error!("There are no input fields in the following URI: {}. It must have one at least.",elem_uri);
        return Err(ApplicationErrors::NoInputFieldURISubject)
    }
    Ok((modified_template, fields))

}

fn parse_predicate_map(tokens: &Vec<String>, init: usize, end: usize) -> ResultApp<Parts>{
    let mut i = init;
    let mut predicate = String::new();
    let mut object_map = Vec::with_capacity(1);

    while i < end {
        if (&tokens[i]).contains("predicate") {
            predicate = tokens[i+ 1].clone();
            i += 1;
        }else if (&tokens[i]).contains("objectMap"){
            if tokens[i + 1].contains('['){
                if let Some(end) = find_closing_bracket(&tokens, i + 1) {
                    let obj = parse_object_map(&tokens, i + 2, end)?;
                    object_map.extend(obj);
                    i = end;
                }
                else{
                    error!("Missing Closing Bracket in a predicate map");
                    return Err(ApplicationErrors::IncorrectMappingFormat)
                }        
            }
            else{
                object_map = vec![Parts::Term(tokens[i + 1].clone())];
                i += 1;
            }
        }else{
            error!("Unknown Token has Appeared in a PredicateMap: {}", &tokens[i]);
            return Err(ApplicationErrors::IncorrectMappingFormat);
        }
        i += 1;
    }


    Ok(Parts::PredicateObjectMap{
        predicate,
        object_map
    })
}

fn parse_object_map(tokens: &Vec<String>, init: usize, end: usize) -> ResultApp<Vec<Parts>>{
    let mut i = init;
    lazy_static!{
        static ref PARENT: Regex = Regex::new("rr:parentTriplesMap").unwrap();
        static ref MAPPING: Regex = Regex::new(r#"<#([a-zA-Z0-9_\-]*)>"#).unwrap();
        static ref JOIN: Regex = Regex::new("rr:joinCondition").unwrap();
        static ref CONSTANT: Regex = Regex::new("rr:constant").unwrap();
        static ref REFERENCE: Regex = Regex::new("rml:reference").unwrap();
        static ref TERMTYPE: Regex = Regex::new("rr:termtype").unwrap();
        static ref DATATYPE: Regex = Regex::new("rr:datatype").unwrap();    
        static ref TEMPLATE: Regex = Regex::new("rr:template").unwrap();    
    };
    if PARENT.is_match(&tokens[i]) || JOIN.is_match(&tokens[i]){
        let mut other_map = String::new();
        let mut join_condition = [String::new(),String::new()];
        while i < end{
            if PARENT.is_match(&tokens[i]){
                if let Some(cap) = MAPPING.captures(&tokens[i + 1]){
                    other_map = cap.get(1).unwrap().as_str().to_string();
                }else{
                    error!("The mapping reference in a parentTriplesMap has an incorrect format. TOKEN: {}", &tokens[i + 1]);
                    return Err(ApplicationErrors::IncorrectMappingFormat)
                }
            }
            else if JOIN.is_match(&tokens[i]){
                let end = find_closing_bracket(&tokens, i + 1).unwrap();
                parse_join_condition(&tokens, i + 2, end, &mut join_condition)?;
            }
            i += 1;
        }  
        return Ok(vec![Parts::ParentTriplesMap{ other_map, join_condition}]);
    }
    else{
        let mut objs = Vec::with_capacity(2);
        while i < end{
            if REFERENCE.is_match(&tokens[i]){
                objs.push(Parts::Reference(tokens[i+1].clone()));
            }
            else if CONSTANT.is_match(&tokens[i]){
                objs.push(Parts::Constant(tokens[i+1].clone()));
            }
            else if DATATYPE.is_match(&tokens[i]){
                objs.push(Parts::DataType(tokens[i+1].clone()));
            }else if TERMTYPE.is_match(&tokens[i]){
                objs.push(Parts::TermType(tokens[i+1].clone()));
            }else if TEMPLATE.is_match(&tokens[i]){
                let (template, input_fields) = parse_input_field(&tokens[i + 1])?;
                objs.push(Parts::Template{
                    template,
                    input_fields,
                });
                i += 1;
            }
            else{
                warning!("An unknown tokens has appeared in the objectMap parser, TOKEN: {}, NEXT TOKEN: {}", &tokens[i], &tokens[i + 1]);
            }
            i += 2;
        }

        return Ok(objs)
    }
}


fn parse_join_condition(tokens: &Vec<String>, init: usize, end: usize, join: &mut [String;2]) -> ResultApp<()>{
    lazy_static!{
        static ref CHILD: Regex = Regex::new("rr:child").unwrap();
        static ref PARENT_CON: Regex = Regex::new("rr:parent").unwrap();
    };
    let mut i = init;
    while i < end{
        if CHILD.is_match(&tokens[i]){
            join[0] = tokens[i + 1].clone();
            i += 1;
        }else if PARENT_CON.is_match(&tokens[i]){
            join[1] = tokens[i + 1].clone();
            i += 1;
        }else{
            error!("JOIN CONDITION ERROR: An unknown token appeared in the join condiction: {}", &tokens[i]);
            return Err(ApplicationErrors::IncorrectMappingFormat)
        }
        i += 1;
    }

    Ok(())
}