
use crate::mappings::AcceptedType;
use crate::{error, info};
use crate::config;
use crate::ResultApp;
use crate::errors::ApplicationErrors;


use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{mpsc};

use std::collections::{HashMap, HashSet};
use jsonpath_lib::selector;
use sxd_xpath::evaluate_xpath;

// Trial and error
const MAX_BATCH: usize = 23; // Number of Insert Queries that are executed by batch

pub fn read_store_data_files(config: &mut config::AppConfiguration, fields: HashMap<PathBuf, HashSet<String>>) -> ResultApp<(rusqlite::Connection, bool)>{
    let mut fi = Vec::new();
    let mut paths = Vec::new();
    
    let files = config.get_data_files();
    for p in files.keys(){
        info!("Opening File: {}", p.display());
        fi.push(fs::File::open(p)?);
        paths.push(p.clone());
    }
    
    let (loc, is_file) = select_storage_loc(&fi, &config)?;
    let (data_tx, data_rx) = mpsc::channel();

    let num_files = files.len();
    let handler = std::thread::spawn(move || -> ResultApp<rusqlite::Connection>{
        store_data(loc, data_rx, num_files)
    });
    create_tables(data_tx.clone(), &files, &fields)?; // This will not fail for sure.
    reading_procedure(config, data_tx, &fields)?;    
    
    match handler.join()?{
        Ok(db) => Ok((db, is_file)),
        Err(error) => Err(error)
    }
}

fn select_storage_loc(fi: &Vec<fs::File>, config: &config::AppConfiguration) -> ResultApp<(&'static str, bool)>{    
    // Get an extimated Size of the datafiles combined. This is more as a guide, given that could be some dupllicate rows that are going to be eliminated.
    let total_memory_usage: usize = fi.iter()
        .map(|file| {
            file.metadata().expect("No Metadata Was Found: Size of File is needed").len() as usize
        })
        .sum();

    let total_memory_usage = total_memory_usage  / 1048576; // To Transform the number of bytes to megabytes (MB) 

    let loc; 
    let is_file;
    if !config.debug_mode() && config.can_be_in_memory_db(total_memory_usage){
        info!("All the files is estimated to requiere {} MB, TMP Database will be created in memory", total_memory_usage);

        loc = ":memory:";
        is_file = false;
    }else{
        info!("All the files is estimated to requiere {} MB, TMP Database will be created in a sqlite DB File", total_memory_usage);
        // If it was already created
        if PathBuf::from("./rossete-tmp").exists(){
            crate::warning!("Previous TMP Storage DB was Found, Proceeding to replace it with new one");
            fs::remove_dir_all("./rossete-tmp")?;
        }
        fs::create_dir("rossete-tmp")?;
        is_file = true;
        loc = "./rossete-tmp/data_tmp.sqlite";
    }

    Ok((loc, is_file))
}

fn store_data(localization: &str, data_rx: mpsc::Receiver<String>, total_files: usize) -> ResultApp<rusqlite::Connection>{
    let conn = rusqlite::Connection::open_with_flags(localization,
        rusqlite::OpenFlags::SQLITE_OPEN_SHARED_CACHE |
        rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX |
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE |
        rusqlite::OpenFlags::SQLITE_OPEN_CREATE)?; // Database Connection
    
    let mut left_files = total_files;
    let mut batch_size = 0;
    let mut buffer = String::with_capacity(10000);
    buffer.push_str("BEGIN; ");
    loop{
        if let Ok(query) = data_rx.recv_timeout(std::time::Duration::from_millis(150)){
            if query.len() == 0{ // Interrumpt
                buffer.push_str(" COMMIT;");
                conn.execute_batch(&buffer)?;
                buffer.clear();
                break
            }
            else if query.len() <= 6{ // THIS CORRESPOND TO THE ID OF THE FILE IN NUMERIC FORM
                info!("File with ID: {} was readed and closed.", query);
                left_files -= 1;
                if left_files == 0{
                    buffer.push_str(" COMMIT;");
                    conn.execute_batch(&buffer)?;
                    buffer.clear();
                    break
                }

            }else if batch_size < MAX_BATCH{
                batch_size += 1;
                buffer.extend(query.chars());
            }
            
            if batch_size == MAX_BATCH{
                buffer.push_str(" COMMIT;");
                conn.execute_batch(&buffer)?;

                buffer.clear();
                buffer.push_str("BEGIN; ");
                batch_size = 0;
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
        if file_type.is_json() || file_type.is_xml(){
   
            let campos = input_fields[file].iter().map(|camp| camp.clone()).collect::<Vec<_>>();
            let iteradores = extract_iterator_and_fields(&campos);
   
            for iter in iteradores.keys(){
                let table_name = get_table_name_with_iterator(file, file_type, iter);
                info!("The following table was created in the database: {}", &table_name);
   
                let mut query = String::with_capacity(1000);
                query.extend(format!("CREATE TABLE {} (\"col_id\" INTEGER PRIMARY KEY AUTOINCREMENT, \"", table_name).chars());
   
                for (i, field) in iteradores[iter].iter().enumerate(){
                    query.extend(field.1.chars());
                    if i != iteradores[iter].len() - 1{
                        query.push_str("\" TEXT, \"");
                    }else{
                        query.push_str("\" TEXT);")
                    }
                }
   
                con.send(query)?;        
            }

        }else{
            let table_name = get_table_name(file, file_type);
            info!("The following table was created in the database: {}", &table_name);
            let mut query = String::with_capacity(1000);
            query.extend(format!("CREATE TABLE {} (\"col_id\" INTEGER PRIMARY KEY AUTOINCREMENT, \"", table_name).chars());
            for (i, field) in input_fields[file].iter().enumerate(){
                query.extend(field.chars());
                if i != input_fields[file].len() - 1{
                    query.push_str("\" TEXT, \"");
                }else{
                    query.push_str("\" TEXT);")
                }
            }
            con.send(query)?;    
        }
    }
    Ok(())
}

fn get_table_name(path: &PathBuf, file_type: &AcceptedType) -> String{
    format!("\"db-{}-{:?}\"", path.file_stem().unwrap().to_str().unwrap(), file_type)
}

fn get_table_name_with_iterator(path: &PathBuf, file_type: &AcceptedType, iterator: &String) -> String{
    format!("\"db-{}-{:?}-{}\"", path.file_stem().unwrap().to_str().unwrap(), file_type, iterator)
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
            let fields = input_fields[&path].iter().map(|field| field.clone()).collect::<Vec<_>>();

            let hand = std::thread::spawn(move || -> ResultApp<()>{
                let file_type = specs.get_file_type();
                if file_type.is_csv(){
                    read_csv(new_id, path, specs, conn, rc, fields)
                }else if file_type.is_tsv(){ // It is the same but it uses tabs
                    read_csv(new_id, path, specs, conn, rc, fields)
                }else if file_type.is_json(){
                    read_json(new_id, path, specs, conn, rc, fields)
                }else if file_type.is_xml(){
                    read_xml(new_id, path, specs, conn, rc, fields)
                }
                else{
                    // No idea Scenario
                    conn.send(format!("{:6}", new_id))?;
                    rc.send(new_id)?;
                    Ok(())
                }
            });

            threads.push(hand);
            threads_id.push(current_file);
            current_file += 1;
        }else{

            // Liberates the reading threads
            let rc = rc_rx.recv()?;
            let thread_id = threads_id.iter().position(|x| x == &rc).expect("Thread ID was not found");

            match threads.remove(thread_id).join()?{
                Ok(_) => {},
                Err(error) => {
                    con.send(String::new())?;
                    return Err(error)
                } 
            }
            threads_id.remove(thread_id);
        }
        if threads.len() == 0 && current_file >= paths.len(){
            break
        }
    }

    Ok(())
}

fn read_csv(id: usize, path: PathBuf, specs: config::FileSpecs, con: mpsc::Sender<String>, rc: mpsc::Sender<usize>, fields: Vec<String>) -> ResultApp<()>{
    // TDOO Given the file type, it creates the reader.
    let table_name = get_table_name(&path, specs.get_file_type());

    let file = fs::File::open(&path)?;
    let encoding = specs.get_encoding();
    let file_reader = encoding_rs_io::DecodeReaderBytesBuilder::new()
                                            .encoding(Some(encoding))
                                            .build(file);

    let mut csv_file = csv::ReaderBuilder::new()
        .delimiter(specs.get_delimiter() as u8)
        .has_headers(specs.get_has_header())
        .from_reader(file_reader);

    let mut initial_query = format!("INSERT INTO {} (\"", table_name);
    for (i, field) in fields.iter().enumerate(){
        initial_query.extend(field.chars());
        if i != fields.len() - 1{
            initial_query.push_str("\", \"")
        }else{
            initial_query.push_str("\") VALUES (\"")
        }
    }

    for row in csv_file.deserialize() {
        let row_data: HashMap<String, String> = match row{
            Ok(d) => d,
            Err(error) => {
                error!("CSV Reader could not extract row from file {}", path.display());
                return Err(error.into());
            }
        };
        let column_names = row_data.keys().collect::<Vec<_>>();
        if !fields.iter().all(|k| column_names.contains(&k)){
            con.send(format!("{:6}", id))?;
            rc.send(id)?;
            error!("There is a missing field in the data file corresponding to the following data table: {}.", table_name);
            eprintln!("This are the columns in the CSV and the requested columns in all the files. Consider changing delimiter in the configuration file to remedy this.");
            column_names.iter().for_each(|c| eprintln!("COLUMN: {}", c));
            fields.iter().for_each(|c| eprintln!("FIELD: {}", c));
            return Err(ApplicationErrors::MissingFieldInData)        
        }

        let data = fields.iter().map(|column| row_data.get(column).expect("There is a missing value in a CSV row").clone()).collect::<Vec<_>>();
        let mut query_buffer = String::with_capacity(fields.iter().map(|f| f.len()).sum::<usize>() + 100);
        query_buffer.extend(initial_query.chars());
        for (i,d) in data.iter().enumerate(){
            query_buffer.extend(d.chars());
            if i != data.len() - 1{
                query_buffer.push_str("\", \"")
            }else{
                query_buffer.push_str("\");")
            }
        }
        con.send(query_buffer)?;
    }

    // DELETE duplicates
    let delete = query_remove_duplicates(&table_name, &fields);
    con.send(delete)?;
    
    // End Transmision.
    con.send(format!("{:6}", id))?;
    rc.send(id)?;
    Ok(())
}

fn read_json(id: usize, path: PathBuf, specs: config::FileSpecs, con: mpsc::Sender<String>, rc: mpsc::Sender<usize>, fields: Vec<String>) -> ResultApp<()>{

    // Read File and get the parsed json.
    let file = fs::File::open(&path)?;
    let encoding = specs.get_encoding();
    let file_reader = encoding_rs_io::DecodeReaderBytesBuilder::new()
                                            .encoding(Some(encoding))
                                            .build(file);


    let json_data: serde_json::Value = serde_json::from_reader(file_reader)?;

    // Divide field by the iterator that uses
    let iter_field = extract_iterator_and_fields(&fields);
    // Iterate by the iterators and get the need data
    
    let mut data_iterator = selector(&json_data);

    for (ref iterator, ref associated_fields) in iter_field{
        let iterable_data = data_iterator(iterator)?;
        let table_name = get_table_name_with_iterator(&path, specs.get_file_type(), iterator);
        for data in iterable_data.iter(){

            let mut field_sel = selector(data);
            let mut init_query = format!("INSERT INTO {} (\"", &table_name); // until the insert of the data
            let mut data_query = String::with_capacity(255);    
            for (i, (field, col)) in associated_fields.iter().enumerate(){
                let retrieven = field_sel(field)?;
                if retrieven.is_empty(){
                    if i == associated_fields.len() - 1{
                        init_query.push_str("\") VALUES (");
                        data_query.push_str(");");
                    }
                    continue
                } 
                let data_retrieven = match to_string_json(retrieven[0]){
                    Some(data) => data,
                    None => {
                        if i == associated_fields.len() - 1{
                            // remove last added chars
                            init_query.pop(); // "
                            init_query.pop(); // _
                            init_query.pop(); // ,

                            data_query.pop(); // _ 
                            data_query.pop(); // ,

                            init_query.push_str(") VALUES (");
                            data_query.push_str(");");
                        }
                        continue    
                    }
                };
                init_query.extend(col.chars());
                data_query.extend(data_retrieven.chars());

                if i != associated_fields.len() - 1{
                    init_query.push_str("\", \"");
                    data_query.push_str(", ");

                }else{
                    init_query.push_str("\") VALUES (");
                    data_query.push_str(");");
                }
            }

            init_query.extend(data_query.chars());
            con.send(init_query)?;
        }
        // Remove duplicates
        let remove_duplicates = query_remove_duplicates(&table_name, &fields);
        con.send(remove_duplicates)?;
    }

    // End Transmission
    con.send(format!("{:6}", id))?;
    rc.send(id)?;
    Ok(())
}

fn read_xml(id: usize, path: PathBuf, specs: config::FileSpecs, con: mpsc::Sender<String>, rc: mpsc::Sender<usize>, fields: Vec<String>) -> ResultApp<()>{
    // TODO Read XML files
    let file = fs::File::open(&path)?;
    let mut xml_string = String::with_capacity(file.metadata().unwrap().len() as usize); 
    let encoding = specs.get_encoding();
    let mut file_reader = encoding_rs_io::DecodeReaderBytesBuilder::new()
                                            .encoding(Some(encoding))
                                            .build(file);


    
    file_reader.read_to_string(&mut xml_string)?;
    let xml_package = sxd_document::parser::parse(&xml_string)?;
    let xml_doc = xml_package.as_document();

    let iter_field = extract_iterator_and_fields(&fields);
    for (iterator, __associated_fields) in iter_field.iter(){
        let __table_name = get_table_name_with_iterator(&path, specs.get_file_type(), iterator);
        let __iterable_data = evaluate_xpath(&xml_doc, iterator)?;        
        // let remove_duplicates = query_remove_duplicates(&table_name, &fields);
        // con.send(remove_duplicates)?;
    }

    // Read File

    
    // Divide field by the iterator that uses
    // Iterate by the iterators and get the need data

    // Remove duplicates

    // End Transmission
    con.send(format!("{:6}", id))?;
    rc.send(id)?;
    Ok(())
}

fn extract_iterator_and_fields(fields: &Vec<String>)  -> HashMap<String, Vec<(String, String)>>{
    let mut iter_field = HashMap::new();

    for f in fields{
        let mut split = f.split("||");
        let iterator = split.next().unwrap().to_string();
        let field = split.next().unwrap().to_string();
        let field = format!("$.{}", field);
        let iter_cap = iter_field.entry(iterator).or_insert(Vec::new());
        iter_cap.push((field, f.clone())); // Field, Column Name
    }
    
    iter_field
    
}


fn query_remove_duplicates(table: &String, fields: &Vec<String>) -> String{
    let mut delete_query = format!("DELETE FROM {} WHERE rowid NOT IN ( SELECT min(rowid) FROM {} GROUP BY \"", &table,&table) ;
    for (i, field) in fields.iter().enumerate(){
        delete_query.extend(field.chars());
        if i != fields.len() - 1{
            delete_query.push_str("\", \"");
        }else{
            delete_query.push_str("\");");
        }
    }
    delete_query

}


fn to_string_json(value: &serde_json::Value) -> Option<String>{
    if value.is_array() || value.is_object() || value.is_null(){
        return None
    }else if value.is_string(){
        return Some(format!("\"{}\"", value.as_str().unwrap()))
    }else if value.is_i64(){
        return Some(format!("\"{}\"", value.as_i64().unwrap()))
    }else if value.is_f64(){
        return Some(format!("\"{}\"", value.as_f64().unwrap()))
    }else if value.is_boolean(){
        return Some(format!("\"{}\"", value.as_bool().unwrap()))
    }else if value.is_u64(){
        return Some(format!("\"{}\"", value.as_u64().unwrap()))
    }else if value.is_number(){
        return Some(format!("\"{}\"", value.as_f64().unwrap()))
    }else{
        return None
    }

}