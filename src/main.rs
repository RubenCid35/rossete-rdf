
mod errors;

type ResultApp<T> = Result<T, errors::ApplicationErrors>;

mod mappings;
mod logging;
mod parser;
mod input;
mod config;
mod search;

use config::AppConfiguration;
use logging::*;
use mappings::maps::Mapping;


use std::path::{self, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet};

use std::time::Instant;

// use clap::{App, Arg};
// use clap::{crate_authors, crate_version, crate_description};

fn main() -> ResultApp<()>{

    // This will be given by the user.
    let output_file = path::PathBuf::from("output.ttl");
    let config_file = path::PathBuf::from("config_example.json");
    let debug = false;

    let mut configuration = config::get_configuration(&output_file, Some(config_file));

    if debug{ // This will be activatedd using a cli flag
        configuration.set_debug_mode();
    }
    if configuration.debug_mode(){
        println!("{:?}", configuration);
    }


    // CLI Input
    let file_name = path::PathBuf::from("./examples/mappings");
    
    // Application Process Controller
    run(configuration, file_name)
}

// Regulates all the processes
fn run(mut config: AppConfiguration, map_path: PathBuf) -> ResultApp<()>{
    let now = Instant::now();
    let prefixes = Arc::new(RwLock::new(HashMap::new()));
    let mappings = parse_all_mappings(&config, map_path, prefixes.clone())?;
    time_info("Parsing Mapping Files", now);
    
    let mut data_fields = HashMap::new();
    add_all_data_files(&mappings, &mut config, &mut data_fields)?;

    let now = Instant::now();
    let _db = input::read_store_data_files(&config, data_fields)?; 
    time_info("Reading and Storing Data Files", now);

    
    Ok(())
}


fn parse_all_mappings(config: &AppConfiguration, mapping_folder: PathBuf, prefixes: Arc<RwLock<HashMap<String, String>>>) -> ResultApp<Vec<Mapping>>{
    // it assumes that all mapping files are encoded in UTF-8;
    let paths = get_all_files(mapping_folder)?;

    let mut mappings = Vec::with_capacity(paths.len());
    let (map_tx, map_rx) = channel();
    let (rc_tx, rc_rx) = channel();

    let max_threads = config.get_parsing_theads();     
    let mut current_path = 0;
    let mut threads = Vec::with_capacity(max_threads as usize);
    let mut threads_id = Vec::with_capacity(max_threads as usize);
    loop{

        // Thread Initialization
        if threads.len() != max_threads && current_path < paths.len(){
            let tx = map_tx.clone();
            let rc_tx_2 = rc_tx.clone();
            let map_file_path = paths[current_path].clone();
            let map_id = current_path;
            let debug = config.debug_mode();
            let pre_c = prefixes.clone();
            let hand = std::thread::spawn(move || -> ResultApp<()>{
                parser::parse_text(map_id as i32 + 1,map_file_path, tx, pre_c, rc_tx_2, debug)
            });
            threads.push(hand);
            threads_id.push(current_path as i32 + 1);
            current_path += 1;
        }
        else{ // Receiving Mappings and Errors.
            let rc = rc_rx.recv()?;
            let idx;
            if rc < 0 {
                idx = threads_id.iter().position(|&id| id == -rc).expect("Failed to Find the parsed thread handler");
                error!("File Could Not Use the following File: {}", paths[(-rc) as usize].display());
                
            }else{
                let tmp = map_rx.recv()?;
                match tmp{
                    Ok(maps) => {
                        mappings.extend(maps);
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
                idx = threads_id.iter().position(|&id| id == rc).expect("Failed to Find the parsed thread handler");
            }
            threads_id.remove(idx);
            threads.remove(idx);
            // info!("THERE ARE {} PARSING PROCESS REMAINING", current_amount);
        }
        if threads.len() == 0 && current_path >= paths.len(){
            break
        }
    }

    Ok(mappings)
}

fn get_all_files(mapping_paths: PathBuf) -> ResultApp<Vec<PathBuf>>{
    if mapping_paths.is_file(){
        return Ok(vec![mapping_paths]);
    }
    let mut map_files = Vec::with_capacity(2);
    for file in std::fs::read_dir(mapping_paths)?{
        let file = file?;
        let path = file.path();

        if path.is_file() && path.extension().unwrap().to_ascii_lowercase() == "ttl"{
            map_files.push(path);
        }

    }
    Ok(map_files)
}

// Add all the data files to the configuration and retrieves all the data fields that need to be accessed
fn add_all_data_files(mappings: &Vec<Mapping>, config: &mut AppConfiguration, fields: &mut HashMap<PathBuf, HashSet<String>>) -> ResultApp<()>{
    for map in mappings.iter() {
        let data_file = map.source_file()?;
        let file_type = map.get_source_file_ext()?;
        config.add_data_file(data_file.clone(), file_type);
        let field = fields.entry(data_file.clone()).or_insert(HashSet::new());
        field.extend(map.get_all_desired_fields());
    }
    Ok(())
}




/*
TODO: Last Item to be added.
TODO: Better flag/option descriptions
    let m = App::new("Rossete RDF Generator")
                .about(crate_description!())
                .version(crate_version!())
                .author(crate_authors!())
                .help_message("Displays this message")
                .arg(
                    Arg::with_name("output")
                    .long("output")
                    .value_name("OUTPUT")
                    .required(true)
                    .takes_value(true)
                    .help("File name where the output file is written")
                )
                .arg(
                    Arg::with_name("config")
                    .long("config")
                    .takes_value(true)
                    .case_insensitive(true)
                    .help("Sets a custom config file to create the main settings of the program")
                    .value_name("FILE")
                )
                .arg(
                    Arg::with_name("mappings")
                    .long("mappings")
                    .required(true)
                    .value_name("MAPPINGS")
                    .takes_value(true)
                    .help("Used mapping in the process of generated rdf. Values: Folder or a file")
                )
                .arg(
                    Arg::with_name("debug")
                    .short("d")
                    .long("debug")
                    .help("Set the debug mode. It displays more information in the intermediary parts")
                    .case_insensitive(true)
                )
                .arg(
                    Arg::with_name("clear")
                    .short("w")
                    .long("clear")
                    .help("Delete the database if it was created while reading the databases")
                )
                .get_matches();
 */