
mod errors;

pub type ResultApp<T> = Result<T, errors::ApplicationErrors>;

pub mod mappings;
pub mod logging;
pub mod parser;

pub use logging::*;

use std::path;
use std::sync::{mpsc, Arc, RwLock};
use std::collections::HashMap;

fn main() -> ResultApp<()>{
    let file_name = path::PathBuf::from("./examples/mappings/rml-mappings.ttl");
    println!("FILE NAME: {}", file_name.display());
    println!("CURRENT DIR: {}", std::env::current_dir().unwrap().display());
    println!("CURRENT EXE: {}", std::env::current_exe().unwrap().display());
    let (transmitter, receiver) = mpsc::channel();
    let prefixes = Arc::new(RwLock::new(HashMap::new()));
    parser::parse_text(file_name, transmitter, prefixes.clone())?;

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

/*
fn main() -> ResultApp<()>{
    use std::io::prelude::*;
    let file = std::fs::File::open(path::PathBuf::from("./examples/mappings/rml-mappings.ttl"))?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines(){
        if let Ok(text) = line{
            println!("{}", text);
        }
    }
    Ok(())
}
*/