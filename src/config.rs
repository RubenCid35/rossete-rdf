
use std::collections;
use std::env;
use std::path::PathBuf;

use crate::ResultApp;
use crate::errors::ApplicationErrors;
use crate::mappings::AcceptedType;
use crate::error;

use encoding_rs::Encoding;

pub struct AppConfiguration{
    // Reading and writing Custom Information.
    file_specs: collections::HashMap<PathBuf, FileSpecs>,
    // max database memory usage: as the total file sizes combine plus 10 Mb.
    memory_threshold: u32, // In MB
    // Max Thread Usage: [Parsing, Reading, Creating RDF]
    threads: [u8;3],
}

impl std::fmt::Debug for AppConfiguration{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Configuration Information: ")?;
        writeln!(f, "------------------------------------------\n")?;
        
        writeln!(f, "Path Information:")?;
        writeln!(f, "------------------------------------------")?;
        writeln!(f, "Current Dir: {}", env::current_dir().unwrap().display())?;
        writeln!(f, "Executable Dir: {}\n", env::current_exe().unwrap().display())?;
        
        writeln!(f, "File Custom Data:")?;
        writeln!(f, "------------------------------------------")?;
        writeln!(f, "Note: Some Information is useless as its delimiter and header in no JSON File")?;
        for (idx, path) in self.file_specs.keys().enumerate(){
            writeln!(f, "File: {}", idx + 1)?;
            writeln!(f, "File Path: {}", path.display())?;
            writeln!(f, "Information: \n{:?}\n", &self.file_specs[path])?;
            writeln!(f, "---------------------------------")?;
        }
        
        writeln!(f, "\nProcess Control (Threads Limit):\n")?;
        writeln!(f, "------------------------------------------")?;
        writeln!(f, "Parsing Maps Max Threads: {}", self.threads[0])?;
        writeln!(f, "Reading Data Max Threads: {}", self.threads[1])?;
        writeln!(f, "Writing RDFS Max Threads: {}", self.threads[2])?;
        
        writeln!(f, "\nDataBase Configuration: ")?;
        writeln!(f, "------------------------------------------")?;
        writeln!(f, "Memory Threshold: {} MB\nClarification: Max Amount of Memory that database is allow to use to be created in memory.", self.memory_threshold)?;

        Ok(())

    }
}

impl std::default::Default for AppConfiguration{
    fn default() -> Self {
        Self{
            file_specs: collections::HashMap::with_capacity(2),
            memory_threshold: 500,
            threads: [5;3]
        }
    }
}

impl AppConfiguration{
    pub fn get_file_config(&self, path: &PathBuf) -> FileSpecs{
        if self.file_specs.contains_key(path){
            self.file_specs[path].clone()
        }else{
            FileSpecs::default()
        }
    }
    pub fn add_file_config(&mut self, path: PathBuf, configuration: FileSpecs){
        if !self.file_specs.contains_key(&path){
            self.file_specs.insert(path, configuration);
        }
    }

    pub fn can_be_in_memory_db(&self, total_memory_usage: u32) -> bool{
        self.memory_threshold <= total_memory_usage
    }

    pub fn from_json(json_data: json::JsonValue) -> ResultApp<Self>{
        let file_data = Self::parse_file_data(&json_data)?;

        Ok(Self::default())
    }
    fn parse_file_data(json_data: &json::JsonValue) -> ResultApp<collections::HashMap<PathBuf, FileSpecs>>{
        let mut file_data: collections::HashMap<PathBuf, FileSpecs> = collections::HashMap::new();
        if json_data.has_key("file_data") && json_data["file_data"].is_array(){
            if let json::JsonValue::Array(files) = &json_data["file_data"] {
                for (i, file) in files.iter().enumerate(){
                    // Obtain path (Obligatory)
                    let path;
                    let mut current_data = FileSpecs::default();
                    if !(file.has_key("path")) && file["path"].is_string(){
                        error!("The File {} In the File Data Configuration Requieres a \"path\" key-value that is a string type", i);
                        return Err(ApplicationErrors::MissingFilePathInConfiguration);
                    }
                    path = PathBuf::from(file["path"].as_str().unwrap());

                    // Type
                    if (file.has_key("type") && file["type"].is_string()){
                        current_data.set_file_type(AcceptedType::from_str(&file["type"].as_str().unwrap().to_lowercase()));
                    }else{
                        if let Some(ext) =  path.extension(){
                            current_data.set_file_type(AcceptedType::from_str(&ext.to_str().unwrap().to_lowercase()));
                        }
                    }
                    // TODO Encoding
                    // TODO Delimiter
                    // TODO Header
                    file_data.insert(path, current_data);
                }
            }else{
                error!("The Files Data in the Configuration must be an Array");
                return Err(ApplicationErrors::InvalidDataEntry);
            }
        }
        Ok(file_data)
    }
}

// File Custom Specs
#[derive(Debug, Clone)]
pub struct FileSpecs{
    // CSV Stuff
    delimiter: char,
    header: u32,
    // Common Stuff
    used_encoding: &'static Encoding,
    file_type: AcceptedType
}

impl std::default::Default for FileSpecs{
    fn default() -> Self {
        Self{
            delimiter: ',',
            header: 0,
            used_encoding: encoding_rs::UTF_8,
            file_type: AcceptedType::CSV
        }
    }
}
impl FileSpecs{
    pub fn get_encoding(&self) -> &Encoding{
        &self.used_encoding
    }
    pub fn get_delimiter(&self) -> char{
        self.delimiter
    }
    
    pub fn get_header_pos(&self) -> u32{
        self.header
    }
    pub fn get_file_type(&self) -> &AcceptedType{
        &self.file_type
    }
    
    pub fn set_delimiter(&mut self, del: char){
        self.delimiter = del;
    }

    pub fn set_header_pos(&mut self, header: u32){
        self.header = header;
    }

    pub fn set_encoding(&mut self, new_encoding: &'static Encoding){
        self.used_encoding = new_encoding;
    }

    pub fn set_file_type(&mut self, file_type: AcceptedType){
        self.file_type = file_type;
    }

    pub fn from_no_csv(&self, encoding: Option<&'static Encoding>, file_type: AcceptedType) -> Self{
        Self{
            delimiter: ' ',
            header: 0,
            used_encoding: encoding.unwrap_or(encoding_rs::UTF_8),
            file_type
        }
    }
}

