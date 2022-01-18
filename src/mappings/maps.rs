use crate::errors::ApplicationErrors;
use super::AcceptedType;
use super::parts;
use crate::ResultApp;

use std::fmt;
use std::path;
use std::sync::Arc;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Mapping{
    components: Vec<parts::Parts>,
    identificador: String,
    prefixes: Arc<HashMap<String, String>> 
}

impl Mapping{
    pub fn new(identificador: String) -> Self{
        Self{
            components: Vec::with_capacity(3),
            identificador,
            prefixes: Arc::new(HashMap::new())
        }
    }


    /// This function allows to check if the mapping is correct according to the number of some type of components
    pub fn is_valid(&self) -> crate::ResultApp<()>{
        let mut n_logical = 0;
        let mut n_subject = 0;
        
        
        for component in &self.components{
            if component.is_predicate(){ // Most common case
                continue
            }else if component.is_logicalsource(){
                n_logical += 1;
            }else if component.is_subjectmap(){
                n_subject += 1
            }else{
                crate::error!("Invalid term appears at the wrong level in  the mapping: {}", self.identificador);
                return Err(ApplicationErrors::ComponentInIncorrectLocation)
            }
        }

        if n_logical != 1{
            crate::error!("Mapping: {}. There is no logical sources in the mapping or there are too many. Current Ammount: {} Valid Ammount: 1", self.identificador, n_logical);
            Err(ApplicationErrors::MissingLogicalSource)
        }else if n_subject != 1{
            crate::error!("Mapping: {}. There is no subject map in the mapping or there are too many. Current Ammount: {} Valid Ammount: 1", self.identificador, n_subject);
            Err(ApplicationErrors::MissingSubjectMap)
        }else{
            Ok(())
        }
    }

    /// Give the specified file in the logicalSource.
    pub fn source_file(&self) -> ResultApp<&path::PathBuf>{
        let source = self.components.iter()
        .find_map(|comp| { 
            match comp{
                parts::Parts::LogicalSource{source, ..} => {
                    Some(source)
                }
                _ => None
            }
        });
        if let Some(file) = source{
            return Ok(file)
        }else{
            return Err(ApplicationErrors::MissingLogicalSource);
        }
    }

    pub fn get_source_file_ext(&self) -> ResultApp<AcceptedType>{
        let source = self.components.iter()
        .find_map(|comp| { 
            match comp{
                parts::Parts::LogicalSource{source: _, reference_formulation, ..} => {
                    Some(reference_formulation)
                }
                _ => None
            }
        });
        match source {
            Some(&file) => Ok(file),
            None => Err(ApplicationErrors::MissingLogicalSource),
        }

    }

    // Request all the fields of the data file that are going to be acessed
    pub fn get_all_desired_fields(&self) -> ResultApp<std::collections::HashSet<String>>{
        let mut fields = std::collections::HashSet::new();
        for element in self.components.iter(){
            let part_fields = element.get_fields();
            fields.extend(part_fields);
        }
        // Append the iterator if data file requieres a path (JSON, XML)
        let iterator = match self.get_logical_source()?{
            parts::Parts::LogicalSource{iterator, ..} => iterator.clone(),
            _ => String::new()
        };

        if iterator.is_empty(){ // CSV / TSV Case (most common)
            return Ok(fields)
        }else{
            let new_fields = fields.iter().map(|field| {
                let mut new_iter = iterator.clone();
                new_iter.push_str("||");
                new_iter.push_str(&field);
                new_iter
            })
            .collect::<std::collections::HashSet<_>>();
            Ok(new_fields)
        }

    }

    pub fn get_identifier(&self) -> &String{
        &self.identificador
    }

    pub fn add_component(&mut self, component: parts::Parts){
        self.components.push(component);
    }

    pub fn change_prefixes(&mut self, prefixes: Arc<HashMap<String, String>>){
        self.prefixes = prefixes
    } 
    
    pub fn get_prefixes(&self) -> Arc<HashMap<String, String>>{
        Arc::clone(&self.prefixes)
    }

    fn get_logical_source(&self) -> ResultApp<&parts::Parts>{
        if let Some(l) = self.components.iter().find(|&p| p.is_logicalsource()){
            Ok(l)
        }else{
            crate::error!("The map {} has no logical source", self.identificador);
            Err(ApplicationErrors::MissingLogicalSource)
        }
    }

    // Returns the name of the table that is asociated to the database
    pub fn get_table_name(&self) -> ResultApp<String>{
        if let Some(parts::Parts::LogicalSource{source, reference_formulation, iterator }) = self.components.iter().find(|&p| p.is_logicalsource()){
            if !iterator.is_empty(){
                Ok(format!("\"db-{}-{:?}-{}\"", source.file_stem().unwrap().to_str().unwrap(), reference_formulation, iterator))
            }else{
                Ok(format!("\"db-{}-{:?}\"", source.file_stem().unwrap().to_str().unwrap(), reference_formulation))
            }
        }else{
            crate::error!("The map {} has no logical source", self.identificador);
            Err(ApplicationErrors::MissingLogicalSource)
        }
    }


}

impl fmt::Debug for Mapping{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "-----------------------------------\nPREFIXES: ")?;
        for (pre, url) in self.prefixes.iter(){
            writeln!(f, "PREFIX: {:<6}\tURL: {:<255}", pre, url)?;
        }

        writeln!(f, "\n<#{}> a rr:TriplesMap", self.identificador)?;
        for comp in &self.components{
            writeln!(f, "\t{:?}", comp)?;
        }
        Ok(())
    }
}

// Para poder guardarlos en un hashset y no tener duplicados.
impl std::hash::Hash for Mapping{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.identificador.hash(state);
    }
}