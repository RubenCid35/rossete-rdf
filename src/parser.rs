
use lazy_static::lazy_static;

use crate::mappings::{
    parts::Parts,
    maps::Mapping
};

use super::ResultApp;
use super::errors::ApplicationErrors;
use crate::{warning, info, error};

use std::path;
use std::fs;
use std::io::prelude::{Read, BufRead};
use std::sync;

use regex::Regex;

/// Main Function that is used to create mapping from a RML File in tbe TTL Format.
pub fn parse_text(file: path::PathBuf, transmitter: sync::mpsc::Sender<ResultApp<Mapping>>) -> ResultApp<()>
{
    // File Reading
    let mut map_file = fs::File::open(file)?;
    let meta = map_file.metadata()?; // Para sacar los metadatos y prelocate memory for the buffer.
    let mut buffer = String::with_capacity(meta.len() as usize);
    map_file.read_to_string(&mut buffer)?;

    // Tokenize the file so we can be parsed.
    let tokens = tokenize(buffer);

    // Get the tokens into diferent mappings and prefixes.
    match parse_tokens(tokens){
        Ok(mappings) => {
            for map in mappings{
                transmitter.send(Ok(map))?;
            }
            return Ok(())    
        }
        Err(error) => {
            transmitter.send(Err(error.clone()))?;
            return Err(error)
        }
    }
}


/// Divide the file into words or tokens so they can be processed quickier.
fn tokenize(text: String) -> Vec<String>{
    text
    .split('\n')
    .flat_map(|sentence| sentence.split(' ').rev().map(|word| word.trim()))
    .filter(|word| word.is_empty())
    .map(|word| word.to_string())
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
fn parse_tokens(tokens: Vec<String>) -> ResultApp<Vec<Mapping>>{

    let mut mappings: Vec<Mapping> = Vec::with_capacity(2);
    let mut prefixes: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
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
            let pre = &tokens[idx + 1][..];
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
            info!("Line: {} Added Prefix: {}", idx, pre);
            prefixes.insert(pre, url);
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
            info!("BASE URI WAS FOUND: {}", &url);
            base_uri = url;
            idx += 1;
        }else if let Some(cap) = MAPPING_INIT.captures(&tokens[idx]){
            let name = cap.get(1).unwrap().as_str().to_string();
            info!("The following parser was found {}. Starting Parsing", &name);
            let map = Mapping::new(name);

            mappings.push(map);
        }else if LOGICALSOURCE.is_match(&tokens[idx]){
            info!("A Logical Source was parsed in the line {}", idx);
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
            info!("A SubjectMap was parsed in the line {}", idx);
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
            info!("A predicateObjectMap was parsed in the line {}", idx);
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
            warning!("An Identified Element has appeared in the Term Index: {}. Term: {}. Last Mapping: {}", idx, &tokens[idx], last_map);
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
