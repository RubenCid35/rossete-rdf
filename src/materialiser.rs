
use crate::ResultApp;
use crate::errors::ApplicationErrors;
use crate::mappings::{
    maps::Mapping,
    parts::Parts
};
use crate::config;
use crate::{warning, error, info}; // Debug and Message Print

use std::io::Write;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::fs;
use std::collections::HashMap;

pub fn rdf_procedure(db: rusqlite::Connection, mappings: Vec<Mapping>, config: config::AppConfiguration) -> ResultApp<()>{

    let db = Arc::new(Mutex::new(db)); // This will make safe to use it between threads.

    let (file_tx, file_rx) = mpsc::channel();
    let config_arc = Arc::new(config);
    let config_arc2 = Arc::clone(&config_arc);
    let num_maps = mappings.len(); 
    let file_thread = thread::spawn( move || {
        write_file(config_arc2, file_rx, num_maps)
    });

    std::thread::sleep(std::time::Duration::from_micros(10)); // Wait a blip

    create_rdf(file_tx, db, mappings, config_arc)?;
    file_thread.join()?
}

fn estabish_conection_between_maps(mappings: &Vec<Mapping>) -> HashMap<String, HashMap<String, (String, String, String)>>{ // parent, child, other_table  
    let mut join_conections = HashMap::with_capacity(mappings.len());
    // TODO Create an easy access table with the conections between maps.
    join_conections
}
fn create_rdf(file_con: mpsc::Sender<Vec<u8>>, db: Arc<Mutex<rusqlite::Connection>>, mappings: Vec<Mapping>, config: Arc<config::AppConfiguration>) -> ResultApp<()>{

    let (rdf_con, rdf_rec) = mpsc::channel::<String>();

    let max_threads = config.get_writing_theads();
    let mut threads: Vec<thread::JoinHandle<Result<(), ApplicationErrors>>> = Vec::with_capacity(max_threads);
    let mut threads_id: Vec<usize> = Vec::with_capacity(max_threads); // It allow us to find the handler of the finished thread

    let (rc_tx, rc_rx) = mpsc::channel::<usize>(); // Indicates which thread has finished to remove it and check if it failed.
    let mut failed_maps: Vec<&str> = Vec::new();
    
    let tables = mappings.iter().map(|map| (map.get_identifier().clone(), map.get_table_name().unwrap())).collect::<HashMap<_, _>>();
    let tables = Arc::new(tables);

    let mut current_map = 0;
    loop{
        // Thread Initialization
        if threads.len() < max_threads && current_map < mappings.len(){
            let rc = rc_tx.clone();
            let rdf_map = mappings[current_map].clone();
            let output_format = config.get_output_format().clone();
            let id = current_map;
            let db_c = Arc::clone(&db);
            let write = file_con.clone();
            let table_co = Arc::clone(&tables);
            let handler = thread::spawn(move || -> ResultApp<()>{
                if output_format.is_nt(){
                    create_rdf_nt(id, rdf_map, rc, db_c, write, table_co)
                }else if output_format.is_ttl(){
                    rc.send(current_map)?;
                    write.send(Vec::new())?;
                    Ok(())
                }else{
                    rc.send(current_map)?;
                    write.send(Vec::new())?;
                    Ok(())
                }
            });

            threads.push(handler);
            threads_id.push(current_map);
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

    drop(tables);
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

fn write_file(config: Arc<config::AppConfiguration>, rdf_rx: mpsc::Receiver<Vec<u8>>, mut num_maps: usize) -> ResultApp<()>{
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
        output_file.write_all(&rdf)?;
        // Stop Condition
        if num_maps == 0{
            break
        }
    }
    Ok(())
}

fn create_rdf_nt(id: usize, map: Mapping, rc: mpsc::Sender<usize>, db: Arc<Mutex<rusqlite::Connection>>, write: mpsc::Sender<Vec<u8>>, tables: Arc<HashMap<String, String>>) -> ResultApp<()>{

    let table_name = map.get_table_name()?;
    println!("RDF FROM DB TABLE: {:<30} AND MAP: {}", &table_name, map.get_identifier());
    let main_columns = map.get_all_desired_fields()?;
    let mut colum_idx = Vec::with_capacity(main_columns.len());
    
    let rows = {
        let fk = db.lock()?;

        // Main columns in the main query.
        let mut columns = String::with_capacity(main_columns.len() * 50);
        for col in main_columns.iter(){
            columns.push('"');
            columns.push_str(&col);
            columns.push_str("\" ,");
        }
        columns.pop();

    
        let select = format!("SELECT {} FROM {}", columns, &table_name);
        println!("{}", select);
        let mut smt = match fk.prepare(&select){
            Ok(s) => s,
            Err(error) => {
                write.send(Vec::new())?;
                rc.send(id)?; 
                return Err(error.into())
            } 
        };

        for col in main_columns.iter(){
            colum_idx.push(smt.column_index(&col).unwrap())
        }
        let raw_rows= smt.query_map([], |row|{
            let mut values: Vec<String> = Vec::with_capacity(main_columns.len());
            for col in colum_idx.iter(){
                values.push(row.get(*col).unwrap_or(String::new()))
            }
            Ok(values)
        })?
        .filter(|row| row.is_ok())
        .map(|row| row.unwrap())
        .collect::<Vec<Vec<String>>>();
        
        raw_rows
    };

    let id_col = main_columns
    .into_iter()
    .map(|key| {
        if key.contains("||"){
            key.split("||").nth(1).unwrap().to_string()
        }else{
            key
        }
    })
    .zip(colum_idx)
    .collect::<HashMap<_, _>>();

    let subject_map = map.get_subject();
    let (temp, input) = if let Parts::Template{template, input_fields} = subject_map.get_template().unwrap(){
        (template, input_fields)
    }else{
        write.send(Vec::new())?;
        rc.send(id)?; 
        return Err(ApplicationErrors::IncorrectMappingFormat)
    };

    let prefixes = map.get_prefixes();

    for val in rows.iter(){
        
        // Getting the subject
        let input_data = 
        input.iter()
        .map(|p|{
            let idx = id_col[p];
            &val[idx]
        }).collect::<Vec<_>>();

        let url = format_uri(temp.clone(), input_data);
        for pre in map.get_predicates(){
            
        }
    }

    // Close and Finish Signal
    write.send(Vec::new())?;
    rc.send(id)?; 
    Ok(())
}

fn format_uri(mut url: String, input: Vec<&String>) -> String{
    let mut input_id = 0;
    while let Some(pos) = url.chars().position(|f| f == '{'){

        url.insert_str(pos, &input[input_id]);

        url.remove(pos + input[input_id].len());
        url.remove(pos + input[input_id].len());
        // This remove the {} symbol.
        input_id += 1;
        if input_id >= input.len(){
            break
        }
    }
    url.insert(0, '<');
    url.push('>');
    url 
}
