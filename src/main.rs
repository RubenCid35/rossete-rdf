
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
use std::collections::HashMap;

use std::time::Instant;

fn main() -> ResultApp<()>{

    let output_file = path::PathBuf::from("output.ttl");
    let config_file = path::PathBuf::from("config_example.json");

    let mut json_tmp = String::with_capacity(1000);
    let mut f = std::fs::File::open(config_file)?;
    f.read_to_string(&mut json_tmp)?;


    let json_config = json::parse(&json_tmp)?;
    let configuration = config::AppConfiguration::from_json(output_file, json_config)?;
    println!("{:?}", configuration);

    let file_name = path::PathBuf::from("./examples/mappings");
    let prefixes = Arc::new(RwLock::new(HashMap::new()));
    let now = Instant::now();
    let mappings = parse_all_mappings(&configuration, file_name, prefixes.clone())?;
    time_info("Parsing Mapping Files", now);
    Ok(())

}

fn parse_all_mappings(config: &AppConfiguration, mapping_folder: PathBuf, prefixes: Arc<RwLock<HashMap<String, String>>>) -> ResultApp<Vec<Mapping>>{
    // it assumes that all mapping files are encoded in UTF-8;
    let paths = get_all_files(mapping_folder)?;

    let mut mappings = Vec::with_capacity(paths.len());
    let (map_tx, map_rx) = channel();
    let (rc_tx, rc_rx) = channel();

    if paths.len() <= config.get_parsing_theads() as usize{
        // Thead Creation
        for (id, path) in paths.iter().enumerate(){
            let tx = map_tx.clone();
            // This will be useless here 
            let rc_tx_2 = rc_tx.clone();
            let map_path = path.clone();
            let pre_c = prefixes.clone();
            std::thread::spawn(move || -> ResultApp<()>{
                parser::parse_text(id as i32, map_path, tx, pre_c, rc_tx_2)
            });
        }
        drop(rc_tx);
        // Reading Data
        for _ in 0..paths.len(){
            let map = map_rx.recv()?;
            let map = map?;
            mappings.extend(map);
        }
    }else{
        let max_threads = config.get_parsing_theads();     
        let mut current_path = 0;
        let mut current_amount = 0;
        let mut threads = Vec::with_capacity(max_threads as usize);
        let mut threads_id = Vec::with_capacity(max_threads as usize);
        loop{
            if current_amount != max_threads && current_path < paths.len(){
                let tx = map_tx.clone();
                // This will be useless here 
                let rc_tx_2 = rc_tx.clone();
                let cp = paths[current_path].clone();
                let cpp = current_path;
                let pre_c = prefixes.clone();
                let hand = std::thread::spawn(move || -> ResultApp<()>{
                    parser::parse_text(cpp as i32 + 1,cp, tx, pre_c, rc_tx_2)
                });
                threads.push(hand);
                threads_id.push(current_path as i32 + 1);
                current_amount += 1;
                current_path += 1;
            }
            else{
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
