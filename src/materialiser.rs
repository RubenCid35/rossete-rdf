
use crate::ResultApp;
use crate::mappings::{
    maps::Mapping,
    parts::Parts
};
use crate::config;
use crate::errors::ApplicationErrors;
use crate::{warning, error, info}; // Debug and Message Print

use core::num;
use std::io::Write;
use std::sync::{Arc, RwLock, mpsc};
use std::collections::HashMap;
use std::thread;
use std::fs;

pub fn rdf_procedure(db: rusqlite::Connection, mappings: Vec<Mapping>, config: config::AppConfiguration) -> ResultApp<()>{

    let (file_tx, file_rx) = mpsc::channel();
    let config_arc = Arc::new(config);
    let config_arc2 = Arc::clone(&config_arc);
    let num_maps = mappings.len(); 
    let file_thread = thread::spawn( move || {
        write_file(config_arc2, file_rx, num_maps)
    });

    create_rdf(file_tx, db, mappings, config_arc)?;
    file_thread.join()?
}

pub fn create_rdf(file_con: mpsc::Sender<&[u8]>, db: rusqlite::Connection, mappings: Vec<Mapping>, config: Arc<config::AppConfiguration>) -> ResultApp<()>{

    let db = Arc::new(RwLock::new(db)); // This is used to request the data

    let (rdf_con, rdf_rec) = mpsc::channel::<String>();

    let max_threads = config.get_writing_theads();
    let mut threads: Vec<thread::JoinHandle<Result<(), ApplicationErrors>>> = Vec::with_capacity(max_threads);
    let mut threads_id: Vec<usize> = Vec::with_capacity(max_threads); // It allow us to find the handler of the finished thread

    let (rc_tx, rc_rx) = mpsc::channel::<usize>(); // Indicates which thread has finished to remove it and check if it failed.
    let mut failed_maps: Vec<&str> = Vec::new();
    
    let mut current_map = 0;
    loop{
        // Thread Initialization
        if threads.len() < config.get_reading_theads() && current_map < mappings.len(){
            let rc = rc_tx.clone();
            let rdf_map = mappings[current_map].clone();
            let output_format = config.get_output_format().clone();
            let id = current_map;
            let db_c = Arc::clone(&db);
            let write = file_con.clone();
            // let handler = thread::spawn(move || -> ResultApp<()>{
            //     if output_format.is_nt(){
            //         create_rdf_nt(id, rdf_map, rc, db_c, write)
            //     }else if output_format.is_ttl(){
            //         Ok(())
            //     }else{
            //         Ok(())
            //     }
            // });

            // threads.push(handler);
            // threads_id.push(current_map);

            rc.send(current_map)?;
            write.send(&[])?;
            current_map += 1;
        }else{
            let rc = rc_rx.recv()?;
            let thread_id = threads_id.iter().position(|x| x == &rc).expect("Thread ID was not found");

            match threads.remove(thread_id).join()?{
                Ok(_) => {},
                Err(error) => {
                    let map_name = &mappings[thread_id].get_identifier();
                    error!("Failed to create RDF using the following mappings: {:<20} Error Code: {:?}", map_name, error);
                    failed_maps.push(map_name);
                },
            }
            threads_id.remove(thread_id);
        }
        
        if threads.len() == 0 && current_map >= mappings.len(){
            break
        }
    }
    if !failed_maps.is_empty(){
        println!("The Program has failed to create rdf from the following maps: ");
        for fail in failed_maps.iter(){
            println!("\t+ {}", fail);
        }
        return Err(ApplicationErrors::FAiledToCreateRDF)
    }else{
        Ok(())
    }

}



fn write_file(config: Arc<config::AppConfiguration>, rdf_rx: mpsc::Receiver<&[u8]>, mut num_maps: usize) -> ResultApp<()>{
    // create a file or if it exist, we remove it first and then we create it again
    let output_path = config.get_output_path();
    if output_path.exists(){
        warning!("It was found a file with the same name as the output file {}, it will be overwrite", output_path.display());
    }
    let mut output_file = fs::OpenOptions::new().write(true).truncate(true).create(true).open(output_path)?;
    loop{
        let rdf = rdf_rx.recv()?;
        if rdf.is_empty(){
            num_maps -= 1;
        }
        // Write Data
        output_file.write_all(rdf)?;
        // Stop Condition
        if num_maps == 0{
            break
        }
    }
    Ok(())
}

fn create_rdf_nt(id: usize, map: Mapping, rc: mpsc::Sender<usize>, db: Arc<RwLock<rusqlite::Connection>>, write: mpsc::Sender<&[u8]>) -> ResultApp<()>{
    let table_name = map.get_table_name()?;
    let buffer = String::new();
    

    rc.send(id)?; // The Confirmation of the end of the writing
    Ok(())
}

