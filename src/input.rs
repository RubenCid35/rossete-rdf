
use crate::mappings::AcceptedType;
use crate::{error, info};
use crate::config;
use crate::ResultApp;
use crate::errors::ApplicationErrors;


use std::fs;
use std::io::{
    BufRead,
    BufReader,
    Read
};
use std::path::PathBuf;
use std::sync::mpsc;

use encoding_rs_io::*;
use sqlite;

use std::collections::{HashMap, HashSet};

pub fn read_store_data_files(config: &config::AppConfiguration, fields: HashMap<PathBuf, HashSet<String>>) -> ResultApp<sqlite::Connection>{
    let mut fi = Vec::new();
    let mut paths = Vec::new();
    
    let files = config.get_data_files();
    for p in files.keys(){
        info!("Opening File: {}", p.display());
        fi.push(fs::File::open(p)?);
        paths.push(p.clone());
    }
    
    let loc = select_storage_loc(&fi, &config)?;
    
    let (data_tx, data_rx) = mpsc::channel();

    let num_files = files.len();
    let handler = std::thread::spawn(move || -> ResultApp<sqlite::Connection>{
        store_data(loc, data_rx, num_files)
    });

    create_tables(data_tx.clone(), &files, &fields)?;
    reading_procedure(config, data_tx, &fields)?;    

    match handler.join() {
        Ok(it) => it,
        Err(err) => return Err(err.into()),
    }
}

fn select_storage_loc(fi: &Vec<fs::File>, config: &config::AppConfiguration) -> ResultApp<&'static str>{    
    // Get the full Size of the datafiles combined.
    let total_memory_usage: usize = fi.iter()
        .map(|file| {
            file.metadata().expect("No Metadata Was Found: Size of File is needed").len() as usize
        })
        .sum();
    let total_memory_usage = total_memory_usage  / 1048576;

    info!("All the files requiered {} MB", total_memory_usage);
    if config.can_be_in_memory_db(total_memory_usage){
        Ok(":memory:")
    }else{
        // If it was already created
        if PathBuf::from("./rossete-tmp").exists(){
            crate::warning!("Previous TMP Storage DB was Found, Proceeding to replace it with new one");
            fs::remove_dir_all("./rossete-tmp")?;
        }
        fs::create_dir("rossete-tmp")?;
        Ok("./rossete-tmp/data_tmp.sqlite")
    }
}

fn store_data(localization: &str, data_rx: mpsc::Receiver<String>, total_files: usize) -> ResultApp<sqlite::Connection>{
    let conn = sqlite::open(localization)?; // Database Connection
    let mut left_files = total_files;
    loop{
        if let Ok(query) = data_rx.recv_timeout(std::time::Duration::from_millis(100)){
            if query.len() <= 6{ // THIS CORRESPOND TO THE ID OF THE FILE IN NUMERIC FORM
                info!("File with ID: {} was successfully readed and stored in the data base", query);
                left_files -= 1;
                if left_files == 0{
                    break
                }
            }else{
                conn.execute(query)?;
            }
            
        }else{
            error!("During the reading of the data files, there were some problems or were too larga and requiered more than 100 ms");
            return Err(ApplicationErrors::DataBaseDidntReceivedData)
        }
    }
    Ok(conn)
}

// Creates all the needed tables from the start to save time.
fn create_tables(con: mpsc::Sender<String>, files: &HashMap<PathBuf, config::FileSpecs>, input_fields: &HashMap<PathBuf, HashSet<String>>) -> ResultApp<()>{
    for file in files.keys(){
        let file_type = files[file].get_file_type();
        let table_name = get_table_name(file, file_type);
        let mut query = String::with_capacity(1000);
        query.extend(format!("CREATE TABLE \"{}\" (id INTEGER PRIMARY KEY", table_name).chars());
        for field in input_fields[file].iter(){
            query.extend(format!(",\n {} TEXT", field).chars())
        }
        query.push_str(");");
        println!("{}", query);
        con.send(query)?;
    }
    Ok(())
}

fn get_table_name(path: &PathBuf, file_type: &AcceptedType) -> String{
    format!("db-{}-{:?}", path.file_stem().unwrap().to_str().unwrap(), file_type)
}


// This function creates and manages all the reading threads of the program.
fn reading_procedure(config: &config::AppConfiguration, con: mpsc::Sender<String>, input_fields: &HashMap<PathBuf, HashSet<String>>) -> ResultApp<()>{
    let files = config.get_data_files();
    let paths = files.keys().collect::<Vec<_>>();
    let mut current_file = 0;
    let mut threads = Vec::with_capacity(config.get_reading_theads());
    let mut threads_id = Vec::with_capacity(config.get_reading_theads());
    let (rc_tx, rc_rx) = mpsc::channel::<usize>();
    loop{
        if threads.len() < config.get_reading_theads() && current_file < files.len(){

            let new_id = current_file;
            let rc = rc_tx.clone();
            let conn = con.clone();
            let path = paths[current_file].clone();
            let specs = files[&path].clone();

            let hand = std::thread::spawn(move || -> ResultApp<()>{
                let file_type = specs.get_file_type();
                // TODO
                if file_type.is_csv(){
                    read_csv(new_id, path, specs, conn, rc)
                }else{
                    read_csv(new_id, path, specs, conn, rc)
                }
            });

            threads.push(hand);
            threads_id.push(current_file);
            current_file += 1;
        }else{

            // Liberates the reading threads
            let rc = rc_rx.recv()?;
            let thread_id = threads_id.iter().position(|x| x == &rc).expect("Thread ID was not found");

            let _ = threads.remove(thread_id).join()??;
            threads_id.remove(thread_id);
        }
        if threads.len() == 0 && current_file >= paths.len(){
            break
        }
    }

    Ok(())
}

fn read_csv(id: usize, path: PathBuf, specs: config::FileSpecs, con: mpsc::Sender<String>, rc: mpsc::Sender<usize>) -> ResultApp<()>{
    // TDOO Given the file type, it creates the reader.

    con.send(format!("{:6}", id))?;
    rc.send(id)?;
    Ok(())
}
