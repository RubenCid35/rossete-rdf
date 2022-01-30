
mod errors;

type ResultApp<T> = Result<T, errors::ApplicationErrors>;

mod mappings;
mod logging;
mod parser;
mod input;
mod config;
mod materialiser;

use config::AppConfiguration;
use logging::*;
use mappings::maps::Mapping;


use std::path::{self, PathBuf};
use std::sync::mpsc::channel;
use std::collections::{HashMap, HashSet};

use std::time::Instant;

// use clap::{App, Arg};
// use clap::{crate_authors, crate_version, crate_description};

const DEBUG: bool = cfg!(debug_assertions);

fn main(){

    // This will be given by the user.
    let output_file = path::PathBuf::from("output.nt");
    let config_file = path::PathBuf::from("config_example.json");

    // ;
    let mut configuration = config::get_configuration(&output_file, Some(config_file)); 

    if DEBUG { // This will be activatedd using a cli flag
        configuration.set_debug_mode();
    }

    // CLI Input
    let file_name = path::PathBuf::from("./examples/mappings");
    
    // Application Process Controller
    match run(configuration, file_name){
        Ok(_) => {
            info!("Process Finished :) :) :)");
        },
        Err(error) => {
            error!("Process Finished Due to an error. ERROR CODE: {:?}", error);
        }
    }
}

// Regulates all the processes
fn run(mut config: AppConfiguration, map_path: PathBuf) -> ResultApp<()>{


    eprintln!("\n");
    info!("Starting to Parse all the given mapping files.");

    let now = Instant::now();
    let mappings = parse_all_mappings(&config, map_path)?;
    time_info("Parsing Mapping Files", now);

    for map in mappings.iter(){
        println!("{:?}", map)
    }

    let mut data_fields = HashMap::new();
    add_all_data_files(&mappings, &mut config, &mut data_fields)?;
    add_all_join_fields(&mappings, &mut data_fields)?;
    

    if config.debug_mode(){ // Display the configuration to see if it is correct
        info!("Showing the Created Configuration");
        eprintln!("{:?}", config);
    }
    
    eprintln!("\n");
    info!("Starting to Read and Store all required data files");
    let now = Instant::now();
    let db = input::read_store_data_files(&config, data_fields)?; 
    time_info("Reading and Storing Data Files", now);

    eprintln!("\n");
    info!("Starting to create the RDF File from Mapping and Data Files");
    let now = Instant::now();
    materialiser::rdf_procedure(db, mappings, config)?;
    time_info("Create RDF File with all Data and Mappings", now);

    Ok(())
}


fn parse_all_mappings(config: &AppConfiguration, mapping_folder: PathBuf) -> ResultApp<Vec<Mapping>>{
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
            let hand = std::thread::spawn(move || -> ResultApp<()>{
                parser::parse_text(map_id as i32 + 1,map_file_path, tx, rc_tx_2, debug)
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
    let mut tmp_files = Vec::with_capacity(mappings.len());
    for map in mappings.iter() {
        let data_file = map.source_file()?;
        let file_type = map.get_source_file_ext()?;
        tmp_files.push((data_file.clone(), file_type));
        let field = fields.entry(data_file.clone()).or_insert(HashSet::new());
        field.extend(map.get_all_desired_fields()?);
    }
    config.remove_unused_files(tmp_files);
    Ok(())
}

fn add_all_join_fields(mappings: &Vec<Mapping>, fields: &mut HashMap<PathBuf, HashSet<String>>) -> ResultApp<()>{
    
    let map_path = mappings.iter().map(|map|{
        let id = map.get_identifier();
        let path = map.source_file().unwrap();
        (id, path)
    }).collect::<HashMap<_, _>>();

    for map in mappings.iter(){
        for (other, field) in map.get_join_fields().unwrap().into_iter(){
            let &p = match map_path.get(&other){
                Some(p) => p,
                None => {
                    return Err(errors::ApplicationErrors::MappingNotFound)
                }
            };
            let other = mappings.iter().find(|&map| map.get_identifier() == &other).unwrap();
            let iterador = other.get_iterator().unwrap();
            let f = fields.entry(p.clone()).or_insert(HashSet::new());
            
            if !iterador.is_empty(){
                f.extend(field
                    .into_iter()
                    .map(|f|{
                        let mut tmp_f = String::with_capacity(iterador.len() + f.len() + 2);
                        tmp_f.push_str(&iterador); 
                        tmp_f.push_str("||");
                        tmp_f.extend(f.chars());
                        tmp_f
                    })
                );
            }
            else{
                f.extend(field.clone())

            }
        }
    }


    Ok(())
}

/*
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
                    .case_insensitive(true)
                    .hidden(true)
                    .help("Set the debug mode. It displays more information in the intermediary parts")
                )
                .arg(
                    Arg::with_name("clear")
                    .short("w")
                    .long("clear")
                    .help("Delete the database if it was created while reading the databases")
                )
                .get_matches();
 */