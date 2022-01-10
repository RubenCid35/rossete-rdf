
mod errors;

type ResultApp<T> = Result<T, errors::ApplicationErrors>;

mod mappings;
mod logging;
mod parsing;
mod input;
mod config;

use config::AppConfiguration;
use logging::*;
use mappings::maps::Mapping;
use parsing::parser;


use std::io::Read;
use std::path::{self, PathBuf};
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet};

use std::time::Instant;

fn main() -> ResultApp<()>{

    let output_file = path::PathBuf::from("output.ttl");
    let config_file = path::PathBuf::from("config_example.json");
    let debug = true;

    let mut json_tmp = String::with_capacity(1000);
    let mut f = std::fs::File::open(config_file)?;
    f.read_to_string(&mut json_tmp)?;


    let json_config = json::parse(&json_tmp)?;
    let mut configuration = config::AppConfiguration::from_json(output_file, json_config)?;
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
    if config.debug_mode(){
        println!("{:?}", config);
        println!("FIELDS TO USE:");
        for path in data_fields.keys(){
            println!("File Path: {}", path.display());
            for field in data_fields[path].iter(){
                println!("\t+  {}", field);
            }
            println!("");
        }

    }

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
    let mut current_amount = 0;
    let mut threads = Vec::with_capacity(max_threads as usize);
    let mut threads_id = Vec::with_capacity(max_threads as usize);
    loop{

        // Thread Initialization
        if current_amount != max_threads && current_path < paths.len(){
            let tx = map_tx.clone();
            // This will be useless here 
            let rc_tx_2 = rc_tx.clone();
            let cp = paths[current_path].clone();
            let cpp = current_path;
            let debug = config.debug_mode();
            let pre_c = prefixes.clone();
            let hand = std::thread::spawn(move || -> ResultApp<()>{
                parser::parse_text(cpp as i32 + 1,cp, tx, pre_c, rc_tx_2, debug)
            });
            threads.push(hand);
            threads_id.push(current_path as i32 + 1);
            current_amount += 1;
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
            current_amount -= 1;
            // info!("THERE ARE {} PARSING PROCESS REMAINING", current_amount);
        }
        if current_amount == 0 && current_path >= paths.len(){
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
        info!("FILE: {} Detected ReferenceFormulation: {}", data_file.display(), file_type);
        config.add_data_file(data_file.clone(), file_type);

        fields.insert(data_file.clone(), map.get_all_desired_fields());
    }
    Ok(())
}