
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
use std::collections::{HashMap, HashSet};

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

    create_rdf(file_tx, Arc::clone(&db), mappings, config_arc)?;
    file_thread.join()?
}

fn create_rdf(file_con: mpsc::Sender<Vec<u8>>, db: Arc<Mutex<rusqlite::Connection>>, mappings: Vec<Mapping>, config: Arc<config::AppConfiguration>) -> ResultApp<()>{

    let max_threads = config.get_writing_theads();
    let mut threads: Vec<thread::JoinHandle<Result<(), ApplicationErrors>>> = Vec::with_capacity(max_threads);
    let mut threads_id: Vec<usize> = Vec::with_capacity(max_threads); // It allow us to find the handler of the finished thread

    let (rc_tx, rc_rx) = mpsc::channel::<usize>(); // Indicates which thread has finished to remove it and check if it failed.
    let mut failed_maps: Vec<&str> = Vec::new();
    
    let tables = mappings.iter().map(|map| {
        let table = map.get_table_name().unwrap();
        let iterador = map.get_iterator().unwrap();
        let template = map.get_subject().get_template().unwrap().clone(); 
        (map.get_identifier().clone(),  (table, iterador, template))
    }).collect::<HashMap<_, _>>();

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
                    create_rdf_ttl(id, rdf_map, rc, db_c, write, table_co)
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
        warning!("The Program has failed to create rdf from the following maps: ");
        for fail in failed_maps.iter(){
            warning!("\t+ {}", fail);
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
        warning!("It was found a file with the same name as the output file \"{}\", it will be overwritten", output_path.display());
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

fn create_rdf_nt<'a>(id: usize, map: Mapping, rc: mpsc::Sender<usize>, db: Arc<Mutex<rusqlite::Connection>>, write: mpsc::Sender<Vec<u8>>, tables: Arc<HashMap<String, (String, String, Parts)>>) -> ResultApp<()>{

    let table_name = map.get_table_name()?;
    info!("RDF FROM DB TABLE: {:<30} AND MAP: {}", &table_name, map.get_identifier());
    let main_columns = map.get_all_desired_fields()?;
    let mut warn = true;
    let (rows, id_col) = match select_main_columns(&table_name, main_columns, &db){
        Ok((row, dict)) => (row, dict),
        Err(error) => {
            error!("Something Ocurred while selecting the main columns from the table {}.", &table_name);
            rc.send(id)?;
            write.send(Vec::new())?;
            return Err(error)
        }
    };
   
    let subject_map = map.get_subject();
    let (temp, input) = if let Parts::Template{template, input_fields} = subject_map.get_template().unwrap(){
        (template, input_fields)
    }else{
        write.send(Vec::new())?;
        rc.send(id)?; 
        return Err(ApplicationErrors::IncorrectMappingFormat)
    };

    // Add the class definition as new predicate
    let class_term = add_definition_predicate(subject_map, &map, &mut warn);

    let predicates = map.get_predicates();

    let join_stmts = generete_join_statements(&map, &predicates, &table_name, &tables);

    let mut buffer = String::with_capacity(1000);
    for val in rows.iter(){
        // Getting the subject
        let url = get_subject(temp, val, input, &id_col);

        if let Some(class) = &class_term{
            buffer.extend(url.chars());
            buffer.extend(class.chars());
            buffer.push_str(".\n");
        }

        for (i, &pre) in predicates.iter().enumerate(){
            buffer.extend(url.chars());

            let term;
            if pre.is_parent(){
                term = term_from_join_object(&map, pre, Arc::clone(&db), val, &id_col, &join_stmts[&i], &mut warn);
            }else{
                term = rdf_term(&map, pre, val,&id_col, &mut warn);
            }
            let term = match term {
                Ok(t) => {
                    if t.is_empty(){ // Remove if data is empty
                        continue
                    }else{
                        t
                    }
                },
                Err(error) => {
                    write.send(Vec::new())?;    
                    rc.send(id)?;
                    return Err(error)
                }
            };
            buffer.extend(term.chars());
            buffer.push_str(".\n");
        }
        buffer.push_str("\n\n");            

        write.send(buffer.bytes().collect())?;
        buffer.clear();
    }

    // Close and Finish Signal
    write.send(Vec::new())?;
    rc.send(id)?; 
    Ok(())
}


fn create_rdf_ttl<'a>(id: usize, map: Mapping, rc: mpsc::Sender<usize>, db: Arc<Mutex<rusqlite::Connection>>, write: mpsc::Sender<Vec<u8>>, tables: Arc<HashMap<String, (String, String, Parts)>>) -> ResultApp<()>{

    let table_name = map.get_table_name()?;
    info!("RDF FROM DB TABLE: {:<30} AND MAP: {}", &table_name, map.get_identifier());
    let main_columns = map.get_all_desired_fields()?;

    let mut warn = true;
    let (rows, id_col) = match select_main_columns(&table_name, main_columns, &db){
        Ok((row, dict)) => (row, dict),
        Err(error) => {
            error!("Something Ocurred while selecting the main columns from the table {}.", &table_name);
            rc.send(id)?;
            write.send(Vec::new())?;
            return Err(error)
        }

    };
   
    let subject_map = map.get_subject();
    let (temp, input) = if let Parts::Template{template, input_fields} = subject_map.get_template().unwrap(){
        (template, input_fields)
    }else{
        write.send(Vec::new())?;
        rc.send(id)?; 
        return Err(ApplicationErrors::IncorrectMappingFormat)
    };

    // Add the class definition as new predicate
    let class_term = add_definition_predicate(subject_map, &map, &mut warn);

    let predicates = map.get_predicates();
    let join_stmts = generete_join_statements(&map, &predicates, &table_name, &tables);
    let mut buffer = String::with_capacity(1000);
    for val in rows.iter(){
        let url = get_subject(temp, val, input, &id_col);
        if let Some(class) = &class_term{
            buffer.extend(url.chars());
            buffer.extend(class.chars());
            if predicates.is_empty(){
                buffer.push_str(".\n");
            }else{
                buffer.push_str(";\n\t\t");
            }
        }

        for (i, &pre) in predicates.iter().enumerate(){
            if i == 0 && class_term.is_none(){
                buffer.extend(url.chars());
            }

            let term;
            if pre.is_parent(){ // CONNECTION BETWEEN MAPS
                term = term_from_join_object(&map, pre, Arc::clone(&db), val, &id_col, &join_stmts[&i], &mut warn);
            }else{
                term = rdf_term(&map, pre, val,&id_col, &mut warn);
            }

            let term = match term {
                Ok(t) => {
                    if t.is_empty(){ // Remove if data is empty
                        continue
                    }else{
                        t
                    }
                },
                Err(error) => {
                    write.send(Vec::new())?;    
                    rc.send(id)?;
                    return Err(error)
                }
            };
            buffer.extend(term.chars());
            if i != predicates.len() - 1{
                buffer.push_str(";\n\t\t");
            }else{
                buffer.push_str(".\n");
            }

        }
        buffer.push_str("\n\n");            

        write.send(buffer.bytes().collect())?;
        buffer.clear();
    }

    // Close and Finish Signal
    write.send(Vec::new())?;
    rc.send(id)?; 
    Ok(())
}

fn generete_join_statements(map: &Mapping, predicates: &Vec<&Parts>, table_name: &String, tables: &Arc<HashMap<String, (String, String, Parts)>>) -> HashMap<usize, (bool, String, String, Vec<String>)>{
    predicates.iter().enumerate().filter(|(_, obj)| obj.is_parent())
    .map(|(i, &obj)|{
        if let Parts::PredicateObjectMap{predicate:_ , object_map} = obj{
            let other_map = object_map.iter().filter(|obj| obj.is_parent()).map(|obj|{
                if let Parts::ParentMap(other) = obj{
                    other.clone()
                }else{
                    String::new()
                }
            }).nth(0).unwrap();
            let iter = map.get_iterator().unwrap();
            let query_table = generate_join_template_query(&table_name, &other_map, &iter, object_map, &tables);

            (i, query_table)
        }else{ // This will never 
            (0, (true, String::new(), String::new(), Vec::new()))
        }
    })
    .collect::<HashMap<_, _>>()
}


fn select_main_columns(table_name: &String, main_columns: HashSet<String>, db: &Arc<Mutex<rusqlite::Connection>>) -> ResultApp<(Vec<Vec<String>>, HashMap<String, usize>)>{
    let mut colum_idx = Vec::with_capacity(main_columns.len());
    let fk = db.lock()?;

    // Main columns in the main query.
    let mut columns = String::with_capacity(main_columns.len() * 50);
    for col in main_columns.iter(){
        columns.push('"');
        columns.push_str(&col);
        columns.push_str("\" ,");
    }
    columns.pop();


    let select = format!("SELECT DISTINCT {0} , CAST(col_id as TEXT) as col_id FROM {1} GROUP BY {0}HAVING MIN(col_id) ORDER BY col_id;", columns, &table_name);
    let mut smt = fk.prepare(&select)?;

    let mut main_columns = main_columns.into_iter().collect::<Vec<_>>();
    main_columns.push("col_id".to_string());

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
    
    let id_col = main_columns
    .into_iter()
    .map(|key| {
        if key.contains("||"){
            key.split("||").nth(1).unwrap().to_string()
        }else{
            key.clone()
        }
    })
    .zip(colum_idx)
    .collect::<HashMap<_, _>>();

    Ok((raw_rows, id_col))
}

fn get_subject(template: &String, val: &Vec<String>, input: &Vec<String>, id_col: &HashMap<String, usize>) -> String{
    // Getting the subject
    let input_data = input.iter()
    .map(|p|{
        let idx = id_col[p];
        val[idx].clone()
    }).collect::<Vec<_>>();
    
    let url = format_uri(template.clone(), &input_data);
    url
}

fn format_uri(mut url: String, input: &Vec<String>) -> String{
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
    url.push(' ');
    url 
}

fn add_definition_predicate(subject: &Parts, map: &Mapping, warn: &mut bool) -> Option<String>{
    if let Parts::SubjectMap{components } = subject{
        let class = components.iter()
        .filter(|comp|{
            if let Parts::Class(_) = comp{
                true
            }else{
                false
            }
        }).nth(0);
        if let Some(Parts::Class(data)) = class{
            let mut definition = "a ".to_string();
            let class_uri = get_predicate(data, map, true, warn); 
            definition.extend(class_uri.chars());
            return Some(definition)

        }else{
            return None
        }

    }
    return None
}



fn rdf_term(map: &Mapping, predicate_part: &Parts, from_table: &Vec<String>, columns: &HashMap<String, usize>, warn: &mut bool) -> ResultApp<String>{
    if let Parts::PredicateObjectMap{predicate, object_map} = predicate_part{
        let mut rdf_term = String::with_capacity(100); 
        
        let pred = get_predicate(predicate, map, true, warn);
        if pred.is_empty(){
            return Ok(rdf_term);
        }
        rdf_term.extend(pred.chars());
       
        let object = term_from_object(map, &object_map, from_table, columns, warn);
        if object.is_empty(){
            return Ok(object)
        }
        rdf_term.extend(object.chars());
        Ok(rdf_term)
    }else{
        Ok(String::new())
    }
}


fn term_from_object(map: &Mapping, objects: &Vec<Parts>, from_table: &Vec<String>, columns: &HashMap<String, usize>, warn: &mut bool) -> String{
    let mut term_kind: bool = true; // true: datatype false: uri
    let mut term_type = String::from("xsd:string");
    let mut object = String::new();

    for element in objects{
        match element{
            Parts::Template{template, input_fields} => {
                term_kind = false;
                let input_data = input_fields.iter()
                .map(|f| columns[f])
                .map(|i| from_table[i].clone())
                .collect::<Vec<_>>();
                
                object = format_uri(template.clone(), &input_data);
                break
            }

            Parts::Reference(obj) => {
                let i = columns[obj];
                object = from_table[i].clone();
                if object.is_empty(){
                    return object
                }
            }
            Parts::DataType(type_data) => {
                term_type = type_data.clone();
            }
            Parts::TermType(type_term) => {
                term_kind = type_term.contains("Literal");
            }
            Parts::ConstantString(obj) => {
                object = obj.clone();
                term_kind = true;
                break
            }
            Parts::ConstantTerm(obj) => {
                object = get_predicate(&obj, map, false, warn);
                term_kind = false;
                break
            }
            _=>{}
        }
    }

    if term_kind{
        let kind = get_predicate(&term_type, map, true, warn);
        return format!("\"{}\"^^{}", object, kind)
    }
    else{
        object.insert(0, '<');
        object.push('>');
        return object
    }

}

fn term_from_join_object(map: &Mapping, pre: &Parts, db: Arc<Mutex<rusqlite::Connection>>, from_table: &Vec<String>, columns: &HashMap<String, usize>, join_data: &(bool, String, String, Vec<String>), warn: &mut bool) -> ResultApp<String>{
    
    let mut data = Vec::new();
    let template = &join_data.2;
    let query = join_data.1.clone();

    // data.push(id);

    
    for f in join_data.3.iter(){
        let i = columns[f];
        let v = &from_table[i];
        data.push(v); 
    }

    // Generate the query.
    let query = add_values_to_query(query, data, &join_data.0);
    if query.is_empty(){
        return Ok(String::new());
    }

    // Request Of the Template input fields.
    let fk = db.lock()?;
    let mut smt = fk.prepare(&query)?;
    
    let max = smt.column_count();
    let row = smt.query_row([], |row| {
        let mut values = Vec::with_capacity(max);
        for k in 0..max{
            let d = row.get(k).unwrap_or(String::new());
            values.push(d);
        }
        Ok(values)
    })?;
    

    let mut term = String::new();

    let predicate = if let Parts::PredicateObjectMap{predicate, ..} = pre{
        predicate
    }else{
        return Ok(term)
    };

    let pred = get_predicate(predicate, map, true, warn);
    if pred.is_empty(){
        return Ok(term);
    }
    term.extend(pred.chars());
   
    let url = format_uri(template.clone(), &row);
    term.extend(url.chars());
    
    Ok(term)
}

fn add_values_to_query(query: String, data: Vec<&String>, same_table: &bool) -> String{
    // let idx = query.chars().enumerate().filter(|(_, c)| *c == '~').map(|(i,_)| i).collect::<Vec<_>>();

    let parts = query.split('~');
    let mut que = String::with_capacity(query.len() + data.len() * 10);
    for (p, d) in parts.zip(data){
        que.push_str(p);
        if *same_table{
            que.push_str(d);
        }else{
            que.push('"');
            que.push_str(d);
            que.push('"');    
        }
    }
    que
}

fn generate_join_template_query(table_name: &String, other_map: &String, map_iterator: &String, objects: &Vec<Parts>, tables: &Arc<HashMap<String, (String, String, Parts)>>) -> (bool, String, String, Vec<String>){
    
    let (other_table, other_iter, other_temp) = &tables[other_map];
    
    let (other_template, input) = if let Parts::Template{template, input_fields} = other_temp{
        let input = input_fields.iter().map(|f|{
            let mut new_field;
            if other_iter.is_empty(){
                new_field = f.clone();
            }else{
                new_field = map_iterator.clone();
                new_field.push_str("||");
                new_field.extend(f.chars());
            }
            new_field
        }).collect::<Vec<_>>();

        (template.clone(), input)
    }else{
        return (true, String::new(), String::new(), Vec::new())
    };

    let mut query = String::with_capacity(255);
    query.push_str("SELECT ");
    for f in input{
        query.push_str(&other_table);
        query.push('.');
        query.push('"');
        query.push_str(&f);
        query.push_str("\", ")
    }

    query.pop();
    query.pop();
    query.pop();
    query.push_str("\" FROM ");
    query.push_str(&other_table);
    let joins;
    if other_table == table_name{
        joins = vec!["col_id".to_string()];
        query.extend(format!(" WHERE {}.col_id == ~", other_table).chars())
    }
    else{
        query.push(',');
        query.push_str(&table_name);
        
        joins = objects.iter()
        .filter(|obj| obj.is_join())
        .map(|obj|{
            match obj{
                Parts::JoinCondition(child, _) => {
                    let mut new_child;
                    if other_iter.is_empty(){
                        new_child = child.clone();
                    }else{
                        new_child = other_iter.clone();
                        new_child.push_str("||");
                        new_child.extend(child.chars());
                    }
                    new_child
                },
                _ =>  String::new()
            }
        }).collect::<Vec<_>>();

        let parents = objects.iter()
        .filter(|obj| obj.is_join())
        .map(|obj|{
            match obj{
                Parts::JoinCondition(_, parent) => {
                    let mut new_parent;
                    if other_iter.is_empty(){
                        new_parent = parent.clone();
                    }else{
                        new_parent = other_iter.clone();
                        new_parent.push_str("||");
                        new_parent.extend(parent.chars());
                    }

                    new_parent
                },
                _ =>  String::new()
            }
        }).collect::<Vec<_>>();

        query.push_str(" WHERE ");
        if !parents.is_empty(){
            // query.push_str(" AND ");
            for (i, parent) in parents.iter().enumerate(){
                query.push_str(&other_table);
                query.push('.');
                query.push('"');
                query.push_str(&parent);
                query.push('"');
                query.push_str(" == ~");
                if i != joins.len() - 1{
                    query.push_str(" AND ")
                }
            }
        }
    }

    query.push(';');
    (other_table == table_name, query, other_template, joins)
}

fn get_predicate(predicate: &String, map: &Mapping, tags: bool, warn: &mut bool) -> String{
    let mut pre = String::with_capacity(predicate.len() + 30);
    let mut parts = predicate.split(':');
    let mut prefix = parts.next().unwrap().to_string();
    if let Some(url) = parts.next(){
        let prefixes = map.get_prefixes();
        prefix.push(':');
        if let Some(pre_url) = prefixes.get(&prefix){
            pre.extend(pre_url.chars());
        }else{
            if *warn{
                warning!("There is a missing prefix definition \"{}\" for the following map: {}", prefix, map.get_identifier());
                *warn = false;
            }
            let mut p = predicate.clone();
            p.push(' ');
            p.push(' ');
            return p
        }
        pre.extend(url.chars());
        if tags{
            pre.insert(0,'<');
            pre.push('>');
            pre.push(' ');
        }
        pre
    }else{
        predicate.clone()
    }
}