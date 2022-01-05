
mod errors;

pub type ResultApp<T> = Result<T, errors::ApplicationErrors>;

pub mod mappings;
pub mod logging;
pub mod parsing;
pub mod input;

pub use logging::*;

use parsing::parser;
use std::path;
use std::sync::{mpsc, Arc, RwLock};
use std::collections::HashMap;

use std::time;

fn main() -> ResultApp<()>{
    let file_name = path::PathBuf::from("./examples/mappings/rml-mappings.ttl");
    
    println!("DEBUG INFORMATION: \nFILE NAME: {}", file_name.display());
    println!("CURRENT DIR: {}", std::env::current_dir().unwrap().display());
    println!("CURRENT EXE: {}\n\n", std::env::current_exe().unwrap().display());

    let now = time::Instant::now();
    let (transmitter, receiver) = mpsc::channel();
    let prefixes = Arc::new(RwLock::new(HashMap::new()));
    parser::parse_text(file_name, transmitter, prefixes.clone())?;

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
