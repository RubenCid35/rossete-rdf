
use std::collections;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;

use crate::ResultApp;
use crate::errors::ApplicationErrors;
use crate::mappings::AcceptedType;
use crate::{error, warning, info, time_info};

use std::io::Read;
use std::time::Instant;

use encoding_rs::{Encoding};

use serde_json;

#[derive(Debug, Clone)]
pub enum OutputFormat{
    NTriplesMap,
    Turtle,
    Other
}
impl OutputFormat{
    pub fn from_str(ext: &str) -> Self{
        if ext == "nt"{
            Self::NTriplesMap
        }else if ext == "ttl"{
            Self::Turtle
        }else{
            Self::Other
        }
    }
    pub fn is_nt(&self) -> bool{
        match self{
            Self::NTriplesMap => true,
            _ => false
        }
    }

    pub fn is_ttl(&self) -> bool{
        match self{
            Self::Turtle => true,
            _ => false
        }
    }
}



pub struct AppConfiguration{
    // Reading and writing Custom Information.
    file_specs: collections::HashMap<PathBuf, FileSpecs>,
    // max database memory usage: as the total file sizes combine plus 10 Mb.
    memory_threshold: usize, // In MB
    // Max Thread Usage: [Parsing, Reading, Creating RDF]
    threads: [usize;3],
    // Output Data
    output_path: PathBuf,
    output_format: OutputFormat,
    // Debug Display
    debug: bool
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
        writeln!(f, "Note: Some Information is useless as its delimiter and header in no CSV Files")?;
        for (idx, path) in self.file_specs.keys().enumerate(){
            writeln!(f, "---------------------------------")?;
            writeln!(f, "File: {}", idx + 1)?;
            writeln!(f, "File Path: {}", path.display())?;
            writeln!(f, "Information: \n{:?}", &self.file_specs[path])?;
        }
        
        writeln!(f, "\nProcess Control (Threads Limit):")?;
        writeln!(f, "------------------------------------------")?;
        writeln!(f, "Parsing Maps Max Threads: {}", self.threads[0])?;
        writeln!(f, "Reading Data Max Threads: {}", self.threads[1])?;
        writeln!(f, "Writing RDFS Max Threads: {}", self.threads[2])?;
        
        writeln!(f, "\nDataBase Configuration: ")?;
        writeln!(f, "------------------------------------------")?;
        writeln!(f, "Memory Threshold: {} MB\nClarification: Max Amount of Memory that database is allow to use to be created in memory.", self.memory_threshold)?;

        writeln!(f, "\nOutput Information: ")?;
        writeln!(f, "------------------------------------------")?;
        writeln!(f, "Output Path: {}", self.output_path.display())?;
        writeln!(f, "Output Format: {:?}", self.output_format)?;
        Ok(())

    }
}


impl AppConfiguration{

    pub fn new(output_path: PathBuf) -> Self{
        let output_format = OutputFormat::from_str(output_path.extension().unwrap().to_str().unwrap_or("nt"));

        Self{
            file_specs: collections::HashMap::with_capacity(2),
            memory_threshold: 500, // 500
            threads: [3;3],
            output_path,
            output_format,
            debug: false
        }
    }
    pub fn get_data_files(&self) -> &collections::HashMap<PathBuf, FileSpecs>{
        &self.file_specs
    }

    pub fn debug_mode(&self) -> bool{
        self.debug
    }
    pub fn set_debug_mode(&mut self){
        self.debug = true;
    }

    pub fn remove_unused_files(&mut self, files: Vec<(PathBuf, AcceptedType)>){

        let tmp = self.file_specs.drain().collect::<HashMap<_, _>>();
        let mut paths= files.iter().map(|(f, _)| f.clone()).collect::<HashSet<_>>();
        let mut final_files = HashMap::with_capacity(files.len());
        for (k, v) in tmp{
            if paths.contains(&k){
                paths.remove(&k);
                final_files.insert(k, v);
            }else {
                continue
            }
        }
        self.file_specs = final_files;

        files.iter()
        .filter(|(f, _)| paths.contains(f))
        .for_each(|(f, t)|{
            self.add_data_file(f.clone(), t.clone())    
        })

    }

    pub fn add_data_file(&mut self, path: PathBuf, file_type: AcceptedType){
        if !self.file_specs.contains_key(&path){
            let file_type2;
            match file_type{
                AcceptedType::Unspecify => {
                    file_type2 = AcceptedType::from_str(&path.extension().expect("File with No extension").to_str().unwrap().to_lowercase())
                }
                _ => {
                    file_type2 = file_type;
                }
            }
            let mut settings = FileSpecs::default();
            settings.set_file_type(file_type2);
            self.file_specs.insert(path, settings);
        }
    }
    pub fn get_output_path(&self) -> &PathBuf{
        &self.output_path
    }

    pub fn get_output_format(&self) -> &OutputFormat{
        &self.output_format
    }

    pub fn get_parsing_theads(&self) -> usize{
        self.threads[0]
    }

    pub fn get_reading_theads(&self) -> usize{
        self.threads[1]
    }

    pub fn get_writing_theads(&self) -> usize{
        self.threads[2]
    }

    pub fn can_be_in_memory_db(&self, total_memory_usage: usize) -> bool{
        self.memory_threshold >= total_memory_usage
    }

    pub fn from_json(output_path: PathBuf, json_data: serde_json::Value) -> ResultApp<Self>{
        let mut tmp = Self::new(output_path);
        tmp.file_specs = Self::parse_file_specs(&json_data)?;
        tmp.threads = Self::parse_threads(&json_data)?;

        if let Some(memory_given) = json_data.get("max-memory-usage"){
            tmp.memory_threshold = match memory_given.as_i64(){
                Some(m) => m as usize,
                None => {
                    error!("The option of \"max memory usage\" must contain a positive integer");
                    return Err(ApplicationErrors::IncorrectJsonFile)
                }
            };
        }
        if let Some(output_format) = json_data.get("output-format"){
            tmp.output_format = match output_format.as_str(){
                Some(format) => OutputFormat::from_str(format),
                None => {
                    error!("The option of \"output format\" must contain a string");
                    return Err(ApplicationErrors::IncorrectJsonFile)

                }
            }
        }
        /*
        if let Some(output_format) = json_data.get("output-encoding"){
            tmp.output_encoding = match output_format.as_str(){
                Some(encoding) => get_encoding_from_str(encoding),
                None => {
                    error!("The option of \"output encoding\" must contain a string");
                    return Err(ApplicationErrors::IncorrectJsonFile)
                }
            }
        }
        */      
        Ok(tmp)
    }

    pub fn parse_file_specs(json_data: &serde_json::Value) -> ResultApp<HashMap<PathBuf, FileSpecs>>{
        let mut specs = HashMap::new();
        
        if let Some(files) = json_data.get("files-data"){
            if !files.is_array(){
                error!("The files-data field must be an array of objects. ");
                return Err(ApplicationErrors::IncorrectFieldType)
            }
            for f in files.as_array().unwrap(){
                let path = if let Some(p) = f.get("path"){
                    PathBuf::from(p.as_str().unwrap())
                }else{
                    warning!("The field \"path\" is requiered in the configuration file to access the file");
                    return Err(ApplicationErrors::MissingFilePathInConfiguration)
                };

                let mut current_spec = FileSpecs::default(); 
                // ENCODING
                if let Some(d) = f.get("encoding"){
                    current_spec.set_encoding(get_encoding_from_str(&d.as_str().unwrap().to_lowercase()));
                }
                // TYPE
                if let Some(d) = f.get("file-type"){
                    current_spec.set_file_type(AcceptedType::from_str(&d.as_str().unwrap().to_lowercase()));
                }
                // Delimiter
                if let Some(d) = f.get("delimiter"){
                    current_spec.set_delimiter(d.as_str().unwrap().chars().nth(0).unwrap());
                }
                // header
                if let Some(d) = f.get("header"){
                    current_spec.set_header(d.as_bool().unwrap());
                }
                specs.insert(path, current_spec);
            }
        }

        Ok(specs)
    }

    pub fn parse_threads(json_data: &serde_json::Value) -> ResultApp<[usize;3]>{
        let mut threads: [usize;3] = [5;3];
        if let Some(data) = json_data.get("threads"){
            if !data.is_object(){
                error!("The Threads Option is requiered to be an object.");
                return Err(ApplicationErrors::IncorrectFieldType)
            }
            if let Some(read) = data.get("reading"){
                let num = read.as_i64().unwrap() as usize;
                threads[1] = num;
            }
            if let Some(parse) = data.get("parsing"){
                let num = parse.as_i64().unwrap() as usize;
                threads[0] = num;
            }
            if let Some(writing) = data.get("writing"){
                let num = writing.as_i64().unwrap() as usize;
                threads[2] = num;
            }
        
        }

        Ok(threads)
    }
}

// File Custom Specs
#[derive(Clone)]
pub struct FileSpecs{
    // CSV Stuff
    delimiter: char,
    has_header: bool,
    // Common Stuff
    used_encoding: &'static Encoding,
    file_type: AcceptedType
}

impl std::fmt::Debug for FileSpecs{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "    -  File Encoding  : {}", self.used_encoding.name())?;
        writeln!(f, "    -  File Type      : {:?}", self.file_type)?;
        writeln!(f, "    +  CSV Related ----------------------------------")?;
        writeln!(f, "    -  Delimiter      : '{}'", self.delimiter)?;
        writeln!(f, "    -  Has Header     : {}", self.has_header)
        
    }
}



impl std::default::Default for FileSpecs{
    fn default() -> Self {
        Self{
            delimiter: ',',
            has_header: true,
            used_encoding: encoding_rs::UTF_8,
            file_type: AcceptedType::CSV
        }
    }
}
impl FileSpecs{

    pub fn get_encoding(&self) -> &&'static Encoding{
        &self.used_encoding
    }
    pub fn get_delimiter(&self) -> char{
        self.delimiter
    }
    
    pub fn get_has_header(&self) -> bool{
        self.has_header
    }
    pub fn get_file_type(&self) -> &AcceptedType{
        &self.file_type
    }
    
    pub fn set_delimiter(&mut self, del: char) -> &mut Self{
        self.delimiter = del;
        self
    }

    pub fn set_header(&mut self, header: bool) -> &mut Self{
        self.has_header = header;
        self
    }

    pub fn set_encoding(&mut self, new_encoding: &'static Encoding) -> &mut Self{
        self.used_encoding = new_encoding;
        self
    }

    pub fn set_file_type(&mut self, file_type: AcceptedType) -> &mut Self{
        self.file_type = file_type;
        let delimiter = match self.file_type{
            AcceptedType::CSV => ',',
            AcceptedType::TSV => '\t',
            _ => ' '
        };
        self.set_delimiter(delimiter);

        self
    }

    /*
    #[allow(dead_code)]
    pub fn from_other(&self, encoding: Option<&'static Encoding>, file_type: AcceptedType) -> Self{
        let delimiter = match file_type{
            AcceptedType::CSV => ',',
            AcceptedType::TSV => '\t',
            _ => ' '
        };

        Self{
            delimiter,
            has_header: false,
            used_encoding: encoding.unwrap_or(encoding_rs::UTF_8),
            file_type
        }
    }
    */
}

// Relates the input text with the same encoding as desired.
fn get_encoding_from_str(value: &str) -> &'static encoding_rs::Encoding{
   match value {
      "BIG5" => encoding_rs::BIG5,
      "EUC-JP" => encoding_rs::EUC_JP,
      "EUC-KR" => encoding_rs::EUC_KR,
      "GB18030" => encoding_rs::GB18030,
      "GBK" => encoding_rs::GBK,
      "IBM866" => encoding_rs::IBM866,
      "ISO-2022-JP" => encoding_rs::ISO_2022_JP,
      "ISO-8859-2" => encoding_rs::ISO_8859_2,
      "ISO-8859-3" => encoding_rs::ISO_8859_3,
      "ISO-8859-4" => encoding_rs::ISO_8859_4,
      "ISO-8859-5" => encoding_rs::ISO_8859_5,
      "ISO-8859-6" => encoding_rs::ISO_8859_6,
      "ISO-8859-7" => encoding_rs::ISO_8859_7,
      "ISO-8859-8" => encoding_rs::ISO_8859_8,
      "ISO-8859-8-I" => encoding_rs::ISO_8859_8_I,
      "ISO-8859-10" => encoding_rs::ISO_8859_10,
      "ISO-8859-13" => encoding_rs::ISO_8859_13,
      "ISO-8859-14" => encoding_rs::ISO_8859_14,
      "ISO-8859-15" => encoding_rs::ISO_8859_15,
      "ISO-8859-16" => encoding_rs::ISO_8859_16,
      "KOI8" => encoding_rs::KOI8_R,
      "KOI8-R" => encoding_rs::KOI8_R,
      "KOI8-U" => encoding_rs::KOI8_U,
      "MACINTOSH" => encoding_rs::MACINTOSH,
      "REPLACEMENT" => encoding_rs::REPLACEMENT,
      "SHIFT_JIS" => encoding_rs::SHIFT_JIS,
      "UTF-8" => encoding_rs::UTF_8,
      "UTF-16" => encoding_rs::UTF_16LE,
      "UTF-16BE" => encoding_rs::UTF_16BE,
      "UTF-16LE" => encoding_rs::UTF_16LE,
      "WINDOWS-874" => encoding_rs::WINDOWS_874,
      "WINDOWS-1250" => encoding_rs::WINDOWS_1250,
      "WINDOWS-1251" => encoding_rs::WINDOWS_1251,
      "WINDOWS-1252" => encoding_rs::WINDOWS_1252,
      "WINDOWS-1253" => encoding_rs::WINDOWS_1253,
      "WINDOWS-1254" => encoding_rs::WINDOWS_1254,
      "WINDOWS-1255" => encoding_rs::WINDOWS_1255,
      "WINDOWS-1256" => encoding_rs::WINDOWS_1256,
      "WINDOWS-1257" => encoding_rs::WINDOWS_1257,
      "WINDOWS-1258" => encoding_rs::WINDOWS_1258,
      "X-MAC-CYRILLIC" => encoding_rs::X_MAC_CYRILLIC,
      "X-USER-DEFINED" => encoding_rs::X_USER_DEFINED,
      _ => encoding_rs::UTF_8
   }
}

pub fn get_configuration(output_file: &PathBuf, config_path: Option<PathBuf>) -> AppConfiguration{
    let now = Instant::now();
    match config_path{
        Some(path) => {
            let mut config_file =  match std::fs::File::open(path){
                Ok(file) => file,
                Err(error) => {
                    error!("Error Trying Opening Configuration File: {:?}", ApplicationErrors::from(error));
                    warning!("Given the error, it will be used the default configuration to proceed with the parsing");
                    return AppConfiguration::new(output_file.clone())
                }
            };

            let mut json_tmp = String::with_capacity(1000);
            if let Err(error) = config_file.read_to_string(&mut json_tmp){
                error!("Error Trying Reading Configuration File: {:?}", ApplicationErrors::from(error));
                warning!("Given the error, it will be used the default configuration to proceed with the parsing");
                return AppConfiguration::new(output_file.clone())
            }
            
            let json_config = match serde_json::from_str(&json_tmp){
                Ok(json) => json,
                Err(error) => {
                    error!("Error parsing text to json values: {:?}", error);
                    warning!("Given the error, it will be used the default configuration to proceed with the parsing");
                    return AppConfiguration::new(output_file.clone())
                }
            };

            match AppConfiguration::from_json(output_file.clone(), json_config){
                Ok(config) => {
                    info!("Given Configuration File was Succesfully Parsed");
                    time_info("Parsing Configuration File", now);
                    config
                },
                Err(error) => {
                    error!("Error parsing the Json Values to the configuration: {:?}", error);
                    warning!("Given the error, it will be used the default configuration to proceed with the parsing");
                    AppConfiguration::new(output_file.clone())
                }
            }
        }
        None => {
            AppConfiguration::new(output_file.clone())
        }
    }
}
/*

    let config_filematch 

    let mut json_tmp = String::with_capacity(1000);
    f.read_to_string(&mut json_tmp)?;

    let now = Instant::now();
    let json_config = json::parse(&json_tmp)?;
    let mut configuration = match config::AppConfiguration::from_json(output_file.clone(), json_config){
        Ok(config) => {
            info!("Given Configuration File was Succesfully Parsed");
            time_info("Parsing Configuration File", now);
            config
        },
        Err(error) => {
            error!("ERROR CODE: {:?}", error);
            warning!("Given the error, it will be used the default configuration to proceed with the parsing");
            AppConfiguration::new(output_file.clone())
        }
    };
    configuration
*/