
mod errors;

type ResultApp<T> = Result<T, errors::ApplicationErrors>;

mod mappings;
mod logging;
mod parsing;
mod input;
mod config;

use logging::*;

use parsing::parser;
use std::io::Read;
use std::path;
use std::sync::mpsc::channel;
use std::sync::{mpsc, Arc, RwLock};
use std::collections::HashMap;

use std::time;

fn main() -> ResultApp<()>{

    let output_file = path::PathBuf::from("output.ttl");
    let config_file = path::PathBuf::from("config_example.json");

    let mut json_tmp = String::with_capacity(1000);
    let mut f = std::fs::File::open(config_file)?;
    f.read_to_string(&mut json_tmp)?;


    let json_config = json::parse(&json_tmp)?;
    let configuration = config::AppConfiguration::from_json(output_file, json_config)?;
    println!("{:?}", configuration);

    let file_name = path::PathBuf::from("./examples/mappings/rml-mappings.ttl");
    let now = time::Instant::now();
    let (transmitter, receiver) = mpsc::channel();
    let (st_tx, st_rc) = channel::<bool>();
    let prefixes = Arc::new(RwLock::new(HashMap::new()));
    let status_tx = st_tx.clone();
    let prefix_zone = prefixes.clone();
    let map_tranmitter = transmitter.clone();
    let hand = std::thread::spawn(move || -> ResultApp<()>{
        parser::parse_text(file_name, map_tranmitter, prefix_zone, status_tx)
    });
    
    time_info("PARSING TEST FILE", now);
    for map in receiver.iter(){
        if let Err(error) = map{
            return Err(error)
        }
        println!("{:?}", map.unwrap());
    }

    println!("\n\nPREFIXES: ");
    let pre = prefixes.read().unwrap();
    for k in pre.keys(){
        println!("PRE -> {}\t\tURL -> {}", k, &pre[k]);
    }


    Ok(())

}
