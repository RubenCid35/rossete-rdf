use crate::errors::ApplicationErrors;
use super::AcceptedType;
use super::parts;
use crate::ResultApp;

use std::fmt;
use std::path;

pub struct Mapping{
    pub components: Vec<parts::Parts>,
    pub identificador: String,
    pub base_uri: String
}

impl Mapping{
    pub fn new(identificador: String) -> Self{
        Self{
            components: Vec::with_capacity(3),
            identificador,
            base_uri: String::new()
        }
    }


    /// This function allows to check if the mapping is correct according to the number of some type of components
    pub fn is_valid(&self) -> crate::ResultApp<()>{
        let mut n_logical = 0;
        let mut n_subject = 0;
        for component in &self.components{
            match component{
                &parts::Parts::LogicalSource{..} => {
                    n_logical += 1;
                }
                &parts::Parts::SubjectMap{..} => {
                    n_subject += 1
                }
                &parts::Parts::PredicateObjectMap{..} => {
                    continue
                }
                _ => {
                    crate::error!("Invalid term appears at the wrong level in  the mapping: {}", self.identificador);
                    return Err(ApplicationErrors::MissingKeyPart)
                }
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
        if let Some(&file) = source{
            return Ok(file)
        }else{
            return Err(ApplicationErrors::MissingLogicalSource);
        }
        
    }

    // Request all the fields of the data file that are going to be acessed
    pub fn get_all_desired_fields(&self) -> std::collections::HashSet<String>{
        let mut fields = std::collections::HashSet::new();
        for element in self.components.iter(){
            let part_fields = element.get_fields();
            fields.extend(part_fields);
        }
        fields
    }

    pub fn add_component(&mut self, component: parts::Parts){
        self.components.push(component);
    }
}

impl fmt::Debug for Mapping{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "<#{}> a rr:TriplesMap", self.identificador)?;
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