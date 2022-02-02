
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
use std::sync::{Arc, mpsc};
use std::collections::HashMap;
use regex::Regex;

/// Main Function that is used to create mapping from a RML File in tbe TTL Format.
pub fn parse_text(id: i32, file: path::PathBuf, transmitter: mpsc::Sender<ResultApp<Vec<Mapping>>>, status_transmitter: mpsc::Sender<i32>, debug: bool) -> ResultApp<()>{
    info!("Parsing File ID: {:2.} PATH: \"{}\"",  id, file.display());
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
    match parse_tokens(tokens, debug){
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

// println!("PHRASE: {}", sentence);
// if let Some(cap) = comment.captures(sentence){
//     println!("COMMENT: {}", sentence);
//     let init = cap.get(1).unwrap().start();
//     if init == 0{
//         String::new()
//     }else{
//         sentence.chars().into_iter().take(init).collect::<String>()
//     }
// }else{
//     sentence.to_string()
// }



// Divide the file into words or tokens so they can be processed quickier.
fn tokenize(text: String) -> Vec<String>{
    // let regex = "(?=(^#.*)|([^<]{1}#[^>].*$))"; // ^[\s\n]?[^(https?:)><|]?#[^>][ \na-zA-Z0-9<>:._/@]+$
    // let comment: Regex = Regex::new(regex).unwrap();
    // COMMENT WITH MAP DEC: <#[0-9a-zA-Z-]*>[0-9a-zA-Z-:;\ ]*(#.*)
    text
    .replace(';', "\n")
    .replace('\r', "\n")
    .split("\n")
    .filter(|&sentence| !sentence.is_empty())
    .map(|sentence|{
        remove_comments(sentence)
    })
    .flat_map(|sentence| sentence.split(' ').map(|word| word.trim().to_string()).collect::<Vec<String>>())
    .flat_map(|sentence| sentence.split(';').map(|word| word.trim().to_string()).collect::<Vec<String>>())
    .flat_map(|mut token| {
        if token.ends_with('[') {
            token.pop();
            vec![token, "[".to_string()]
        }else{
            vec![token]
        }
    })
    .flat_map(|mut token| {
        if token.ends_with(']') && !token.contains('['){
            token.pop();
            vec![token, "]".to_string()]
        }else{
            vec![token]
        }
    })
    .filter(|word| !word.is_empty())
    .collect()
}

fn remove_comments(sentence: &str) -> String{
    let mut phrase = String::with_capacity(sentence.len());

    let sentence = sentence.chars().collect::<Vec<_>>();

    let mut in_uri = false;
    for i in 0..sentence.len(){
        let c = sentence[i];
        if !in_uri && c == '#'{
            if i == 0{
                break
            }else if sentence[i - 1] == '<'{
                phrase.push(c);
            }else if i != phrase.len() - 1 && sentence[i + 1] == '>'{
                phrase.push(c);
            }
            else{
                break
            }
        }else if c == '<' || c == '>'{
            in_uri = !in_uri;
            phrase.push(c);
        }else{
            phrase.push(c);
        }
    }
    phrase
}


// Get the index of the closing bracket if there is.
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
        if map_str[close].contains('['){
            closing += 1;
        }
        if map_str[close].contains(']'){
            if closing == 1{
                return Some(close);
            }else{
                closing -= 1;
            }
        }
        
        close += 1;
    }
}

/// Parse the main 5 Mapping Parts and create all the mappings and prefixes.
fn parse_tokens(tokens: Vec<String>, debug: bool) -> ResultApp<Vec<Mapping>>{

    let mut mappings: Vec<Mapping> = Vec::with_capacity(2);
    let mut prefixes = HashMap::new();
    // let mut prefixes: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut idx = 0;
    let mut last_map_name: String = String::new();
    while idx < tokens.len(){
        lazy_static!{
            static ref PREFIX: Regex = Regex::new(r#"@(prefix|PREFIX)"#).unwrap();
            static ref PREFIX_URL: Regex = Regex::new(r#"<(https?://[a-zA-Z0-9:\.\#/_\-]{0,256})>\s*\.?"#).unwrap();
            static ref BASE: Regex = Regex::new(r#"@(base|BASE)"#).unwrap();
            
            static ref MAPPING_INIT: Regex = Regex::new(r#"<#([a-zA-Z0-9_\-]*)>"#).unwrap();
            static ref MAPDECLARATION: Regex = Regex::new("(rdf:type|a)").unwrap();
            static ref MAPTYPE: Regex = Regex::new("rr:TriplesMap").unwrap();

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
                        error!("Error While Formatting the URL in the PREFIXES: URL: {}", &tokens[idx + 2]);
                        return Err(ApplicationErrors::IncorrectMappingFormat);    
                    }
                }
            };
            prefixes.insert(pre, url);
            // Se manda a una zona central.

            // prefixes.insert(pre, url);
            idx += 2;
        }
        else if BASE.is_match(&tokens[idx]) && (idx + 1) < tokens.len(){
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
            prefixes.insert(String::new(), url);
            idx += 1;
        }
        else if let Some(cap) = MAPPING_INIT.captures(&tokens[idx]){
            let name = cap.get(1).unwrap().as_str().to_string();
            if debug{
                info!("The following mapping was found {}. Parsing Has Started", &name);
            }
            let map = Mapping::new(name);
            mappings.push(map);
            last_map_name = mappings[mappings.len() - 1].get_identifier().clone();
        }
        else if LOGICALSOURCE.is_match(&tokens[idx]){
            if !&tokens[idx + 1].contains('['){
                error!("The Mapping {} requires at least a rml:source component", &last_map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat)
            }
            else if let Some(finish) = find_closing_bracket(&tokens, idx + 1){
                let logicalsource = parse_logical_source(&tokens, idx + 2, finish, &last_map_name)?;
                mappings.last_mut().unwrap().add_component(logicalsource);
                idx = finish;
            }
            else{
                error!("In the Mapping {}, the logicalSource requieres ] at some point to finish the statement", &last_map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat);     
            }

        }
        else if SUBJECTMAP.is_match(&tokens[idx]){
            if !&tokens[idx + 1].contains('['){
                error!("The Mapping {} requires at least a rr:template component", &last_map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat)
            }
            else if let Some(finish) = find_closing_bracket(&tokens, idx + 1){
                let subject_map = parse_subject_map(&tokens, idx + 2, finish, &last_map_name)?;
                mappings.last_mut().unwrap().add_component(subject_map);
                idx = finish;
            }
            else{
                error!("In the Mapping {}, the subjectMap requieres ] at some point to finish the statement", &last_map_name);
                return Err(ApplicationErrors::IncorrectMappingFormat);     
            }

        }
        else if PREDICATEOBJECTMAP.is_match(&tokens[idx]){
            ////info!("A predicateObjectMap was parsed in the line {}", idx);
            if !&tokens[idx + 1].contains('['){
                error!("In the Mapping {} (last token id: {:.3}), the rr:predicateObjectMap requires at least a rr:predicate and rr:objectMap component", &last_map_name, idx);
                return Err(ApplicationErrors::MissingClosingBracket)
            }
            else if let Some(finish) = find_closing_bracket(&tokens, idx + 1){
                let predicate_map = parse_predicate_map(&tokens, idx + 2, finish, &last_map_name)?;
                mappings.last_mut().unwrap().add_component(predicate_map);
                idx = finish;
            }
            else{
                error!("In the Mapping {} (last token id: {:.3}), the rr:predicateObjectMap requieres ] at some point to finish the statement", &last_map_name, idx);
                return Err(ApplicationErrors::MissingClosingBracket);     
            }   
        }
        else if MAPDECLARATION.is_match(&tokens[idx]) && MAPTYPE.is_match(&tokens[idx + 1]){
            idx += 1; // Do Nothing            
        }
        else{
            // To get the last map identification
            let last_map = match mappings.last(){
                Some(map) => {
                    map.get_identifier()
                }
                None => "No Map Was Created"
            };
            warning!("An Unidentified Element has appeared in the Term Index: {}. Term: {}. Last Mapping: {} Last Token: {}", idx, &tokens[idx], last_map, &tokens[idx - 1]);
        }
        
        idx += 1;
    }
    
    let prefix_arc = std::sync::Arc::new(prefixes);
    // Check if all the map have the requiered components: logicalSource and SubjectMap.
    for map in mappings.iter_mut(){
        if let Err(error) = map.is_valid(){
            return Err(error)
        }
        // Add the reference to the prefixes
        map.change_prefixes(Arc::clone(&prefix_arc));
    }
    Ok(mappings)

}


// --------- Component Parsing ---------------
fn parse_logical_source(tokens: &Vec<String>, init: usize, end: usize, last_map: &str) -> ResultApp<Parts>{
    let mut idx = init;
    let mut file_path = path::PathBuf::new();
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
            let p = tokens[idx + 1].replace('"', "");
            file_path = path::PathBuf::from(&p);
            if let Some(ext) = file_path.extension(){
                file_type = AcceptedType::from_str(ext.to_str().unwrap());
            }
            idx += 1;
        }else if ITERATOR.is_match(&tokens[idx]){
            iterator = tokens[idx + 1].replace('"', "");
            idx += 1;
        }else if IS_FILE_TYPE.is_match(&tokens[idx]){
            if let Some(cap) = FILE_TYPE.captures(&tokens[idx + 1]){
                let new_type = AcceptedType::from_str(&cap.get(1).unwrap().as_str().to_lowercase());
                if !(new_type.is_csv() && file_type.is_tsv()){
                    file_type = new_type;
                }
            }
            idx += 1;
        }else{
            warning!("Some unknown tokens has appeared in the logicalSource, TOKEN: {} LAST MAP: {}", &tokens[idx], last_map)
        }

        idx += 1;
    }



    Ok(Parts::LogicalSource{
        source: file_path,
        reference_formulation: file_type,
        iterator
    })
}

fn parse_subject_map(tokens: &Vec<String>, init: usize, end: usize, last_map: &str) -> ResultApp<Parts>{
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
            let (template, input_fields) = parse_input_field(&tokens[idx + 1], last_map)?;
            comps.push(Parts::Template{
                template,
                input_fields,
            });
            idx += 1; 
        }
        else if GRAPHMAP.is_match(&tokens[idx]){
            let comp: Parts;
            if CONSTANT.is_match(&tokens[idx + 1]){
                if tokens[idx + 2].contains('"'){
                    comp = Parts::ConstantString(tokens[idx + 2].replace('"', ""));
                }else{
                    comp = Parts::ConstantTerm(tokens[idx + 2].clone());
                }
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

fn parse_input_field(elem_uri: &str, last_map: &str) -> ResultApp<(String, Vec<String>)>{
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
        error!("There are no input fields in the following URI: {}. It must have one at least. LAST MAP: {}",elem_uri, last_map);
        return Err(ApplicationErrors::NoInputFieldURISubject)
    }
    Ok((modified_template, fields))

}

fn parse_predicate_map(tokens: &Vec<String>, init: usize, end: usize, last_map: &str) -> ResultApp<Parts>{
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
                    let obj = parse_object_map(&tokens, i + 2, end, &last_map)?;
                    object_map.extend(obj);
                    i = end;
                }
                else{
                    error!("Missing Closing Bracket in a predicate map in this map: {}", last_map);
                    return Err(ApplicationErrors::MissingClosingBracket)
                }        
            }
            else{
                object_map = vec![Parts::Term(tokens[i + 1].clone())];
                i += 1;
            }
        }else{
            error!("Unknown Token has Appeared in a PredicateMap: {} LAST MAP: {}", &tokens[i], last_map);
            return Err(ApplicationErrors::IncorrectMappingFormat);
        }
        i += 1;
    }


    Ok(Parts::PredicateObjectMap{
        predicate,
        object_map
    })
}

fn parse_object_map(tokens: &Vec<String>, init: usize, end: usize, last_map: &str) -> ResultApp<Vec<Parts>>{
    let mut i = init;
    lazy_static!{
        static ref PARENT: Regex = Regex::new("rr:parentTriplesMap").unwrap();
        static ref MAPPING: Regex = Regex::new(r#"<#([a-zA-Z0-9_\-]*)>"#).unwrap();
        static ref JOIN: Regex = Regex::new("rr:joinCondition").unwrap();
        static ref CONSTANT: Regex = Regex::new("rr:constant").unwrap();
        static ref REFERENCE: Regex = Regex::new("rml:reference").unwrap();
        static ref TERMTYPE: Regex = Regex::new("rr:term[tT]ype").unwrap();
        static ref DATATYPE: Regex = Regex::new("rr:data[tT]ype").unwrap();    
        static ref TEMPLATE: Regex = Regex::new("rr:template").unwrap();    
    };

    let mut objs = Vec::with_capacity(2);
    while i < end{
        if REFERENCE.is_match(&tokens[i]){
            objs.push(Parts::Reference(tokens[i+1].replace('"', "")));
            i += 1;
        }
        else if CONSTANT.is_match(&tokens[i]){
            if tokens[i+1].contains('"'){
                objs.push(Parts::ConstantString(tokens[i+1].replace('"',"")));
            }else{
                objs.push(Parts::ConstantTerm(tokens[i+1].clone()));
            }
            i += 1;
        }
        else if DATATYPE.is_match(&tokens[i]){
            objs.push(Parts::DataType(tokens[i+1].clone()));
            i += 1;
        }else if TERMTYPE.is_match(&tokens[i]){
            objs.push(Parts::TermType(tokens[i+1].clone()));
            i += 1;
        }else if TEMPLATE.is_match(&tokens[i]){
            let (template, input_fields) = parse_input_field(&tokens[i + 1], last_map)?;
            objs.push(Parts::Template{
                template,
                input_fields,
            });
            i += 1;
        }else if PARENT.is_match(&tokens[i]){
            if let Some(cap) = MAPPING.captures(&tokens[i + 1]){
                let other_map = cap.get(1).unwrap().as_str().to_string();
                objs.push(Parts::ParentMap(other_map));
                i += 1;
            }else{
                error!("The mapping reference in a parentTriplesMap has an incorrect format. TOKEN: {} LAST MAP: {}", &tokens[i + 1], last_map);
                return Err(ApplicationErrors::IncorrectMappingFormat)        
            }
        }else if JOIN.is_match(&tokens[i]) {
            if tokens[i + 1].contains('['){
                if let Some(end) = find_closing_bracket(&tokens, i + 1){
                    let join = parse_join_condition(&tokens, i + 2, end, last_map)?;
                    objs.push(join);  
                    i = end;
                }else{
                    error!("The mapping reference in a joinCondition has a missing closing bracket. TOKEN: {} LAST MAP: {}", &tokens[i + 1], last_map);
                    return Err(ApplicationErrors::MissingClosingBracket)        
                }
            }else{
                error!("The joinCondition clause in the map {1} has an invalid structure. TOKEN: {0} LAST MAP: {1}", &tokens[i + 1], last_map);
                return Err(ApplicationErrors::IncorrectMappingFormat)        
            }
        }
        else{
            warning!("An unknown tokens has appeared in the objectMap parser, TOKEN: {}, NEXT TOKEN: {}, LAST MAP: {}", &tokens[i], &tokens[i + 1], last_map);
        }
        i += 1;
    }
    return Ok(objs)
}

fn parse_join_condition(tokens: &Vec<String>, init: usize, end: usize, last_map: &str) -> ResultApp<Parts>{
    lazy_static!{
        static ref CHILD: Regex = Regex::new("rr:child").unwrap();
        static ref PARENT_CON: Regex = Regex::new("rr:parent").unwrap();
    };
    let mut i = init;
    let mut child = String::new();
    let mut parent = String::new();
    while i < end{
        if CHILD.is_match(&tokens[i]){
            child = tokens[i + 1].replace('"', "");
            i += 1;
        }else if PARENT_CON.is_match(&tokens[i]){
            parent = tokens[i + 1].replace('"', "");
            i += 1;
        }else{
            error!("JOIN CONDITION ERROR: An unknown token appeared in the join condiction: {} LAST MAP: {}", &tokens[i], last_map);
            return Err(ApplicationErrors::IncorrectMappingFormat)
        }
        i += 1;
    }

    Ok(Parts::JoinCondition(child, parent))
}


#[cfg(test)]
mod test_parser{
    use super::remove_comments;

    #[test]
    fn remove_comment_simple(){
        let test = "# hola mundo";
        let result = remove_comments(test);
        assert_eq!(&result, "");
    }

    #[test]
    fn remove_uri(){
        let test = "@prefix wgs84_pos: <http://www.w3.org/2003/01/geo/wgs84_pos#lat>.";
        let result = remove_comments(test);
        assert_eq!(&result, test);
    }

    #[test]
    fn remove_uri_comment(){
        let test = "@prefix xsd: <http://www.w3.org/2001/XMLSchema#>. # Esto debe borrarse";
        let result = remove_comments(test);
        assert_eq!(&result, "@prefix xsd: <http://www.w3.org/2001/XMLSchema#>. ");
    }

    #[test]
    fn do_nothing(){
        let test = "rr:predicateObjectMap [";
        let result = remove_comments(test);
        assert_eq!(&result, test);
    }
}