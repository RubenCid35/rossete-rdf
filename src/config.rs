#![allow(dead_code)]

use std::collections;
use std::env;
use std::path::PathBuf;

use crate::ResultApp;
use crate::errors::ApplicationErrors;
use crate::mappings::AcceptedType;
use crate::error;

use encoding_rs::Encoding;

#[derive(Debug)]
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
}


pub struct AppConfiguration{
    // Reading and writing Custom Information.
    file_specs: collections::HashMap<PathBuf, FileSpecs>,
    // max database memory usage: as the total file sizes combine plus 10 Mb.
    memory_threshold: u32, // In MB
    // Max Thread Usage: [Parsing, Reading, Creating RDF]
    threads: [u8;3],
    // Output Data
    output_encoding: &'static Encoding,
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
            writeln!(f, "File: {}", idx + 1)?;
            writeln!(f, "File Path: {}", path.display())?;
            writeln!(f, "Information: \n{:?}", &self.file_specs[path])?;
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

        writeln!(f, "\nOutput Information: ")?;
        writeln!(f, "------------------------------------------")?;
        writeln!(f, "Output Path: {}", self.output_path.display())?;
        writeln!(f, "Output Format: {:?}", self.output_format)?;
        writeln!(f, "Output Encoding: {}", self.output_encoding.name())?;
        Ok(())

    }
}


impl AppConfiguration{
    pub fn new(output_path: PathBuf) -> Self{
        let output_format = OutputFormat::from_str(output_path.extension().unwrap().to_str().unwrap_or("nt"));

        Self{
            file_specs: collections::HashMap::with_capacity(2),
            memory_threshold: 500,
            threads: [5;3],
            output_encoding: encoding_rs::UTF_8,
            output_path,
            output_format,
            debug: false
        }
    }
    pub fn debug_mode(&self) -> bool{
        self.debug
    }
    pub fn set_debug_mode(&mut self){
        self.debug = true;
    }

    pub fn get_data_file(&self, path: &PathBuf) -> FileSpecs{
        if self.file_specs.contains_key(path){
            self.file_specs[path].clone()
        }else{
            FileSpecs::default()
        }
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
    pub fn get_parsing_theads(&self) -> u8{
        self.threads[0]
    }

    pub fn get_reading_theads(&self) -> u8{
        self.threads[1]
    }

    pub fn get_writing_theads(&self) -> u8{
        self.threads[2]
    }

    pub fn can_be_in_memory_db(&self, total_memory_usage: u32) -> bool{
        self.memory_threshold <= total_memory_usage
    }

    pub fn from_json(output_path: PathBuf, json_data: json::JsonValue) -> ResultApp<Self>{
        let mut tmp = Self::new(output_path);
        
        tmp.file_specs = Self::parse_file_data(&json_data)?;
        
        if json_data.has_key("max-memory-usage") && json_data["max-memory-usage"].is_number(){
            tmp.memory_threshold = json_data["max-memory-usage"].as_u32().unwrap_or(500);
        }
        
        if json_data.has_key("output-encoding") && json_data["output-encoding"].is_string(){
            let enc = json_data["output-encoding"].as_str().unwrap().to_uppercase();
            tmp.output_encoding = get_encoding_from_str(&enc);
        }
        if json_data.has_key("output-format") && json_data["output-format"].is_string(){
            tmp.output_format = OutputFormat::from_str(&json_data["output-format"].as_str().unwrap().to_lowercase());
        }
        if json_data.has_key("threads") && json_data["threads"].is_object(){
            let threads = &json_data["threads"];
            let mut used_threads = [1;3];
            if threads.has_key("reading") && threads["reading"].is_number(){
                used_threads[1] = threads["reading"].as_u8().unwrap();
            }
            if threads.has_key("parsing") && threads["parsing"].is_number(){
                used_threads[0] = threads["parsing"].as_u8().unwrap();
            }
            if threads.has_key("writting") && threads["writting"].is_number(){
                used_threads[2] = threads["writting"].as_u8().unwrap()
            }
            tmp.threads = used_threads;
        }   

        Ok(tmp)
    }
    fn parse_file_data(json_data: &json::JsonValue) -> ResultApp<collections::HashMap<PathBuf, FileSpecs>>{
        let mut file_data: collections::HashMap<PathBuf, FileSpecs> = collections::HashMap::new();
        if json_data.has_key("files-data") && json_data["files-data"].is_array(){
            if let json::JsonValue::Array(files) = &json_data["files-data"] {
                for (i, file) in files.iter().enumerate(){
                    // Obtain path (Obligatory)
                    let path;
                    let mut current_data = FileSpecs::default();
                    if !file.has_key("path") || !file["path"].is_string(){
                        error!("The File {} In the File Data Configuration Requieres a \"path\" key-value that is a string type", i);
                        return Err(ApplicationErrors::MissingFilePathInConfiguration);
                    }
                    path = PathBuf::from(file["path"].as_str().unwrap());

                    // Type
                    if file.has_key("type") && file["type"].is_string(){
                        current_data.set_file_type(AcceptedType::from_str(&file["type"].as_str().unwrap().to_lowercase()));
                    }else{
                        if let Some(ext) =  path.extension(){
                            current_data.set_file_type(AcceptedType::from_str(&ext.to_str().unwrap().to_lowercase()));
                        }
                    }

                    if file.has_key("encoding") && file["encoding"].is_string(){
                        let enc = &file["encoding"].as_str().unwrap().to_uppercase();
                        current_data.set_encoding(get_encoding_from_str(enc));
                    }
                    if file.has_key("delimiter") && file["delimiter"].is_string(){
                        let del = file["delimiter"].as_str().unwrap().chars().next().unwrap_or(',');
                        current_data.set_delimiter(del);
                    }
                    if file.has_key("header") && file["header"].is_number(){
                        let header = file["header"].as_u32().unwrap_or(0);
                        current_data.set_header_pos(header);
                    }
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
#[derive(Clone)]
pub struct FileSpecs{
    // CSV Stuff
    delimiter: char,
    header: u32,
    // Common Stuff
    used_encoding: &'static Encoding,
    file_type: AcceptedType
}

impl std::fmt::Debug for FileSpecs{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "    -  File Encoding  : {}", self.used_encoding.name())?;
        writeln!(f, "    -  File Type      : {:?}", self.file_type)?;
        writeln!(f, "    +  CSV Related ----------------------------------")?;
        writeln!(f, "    -  Delimiter      : {}", self.delimiter)?;
        writeln!(f, "    -  Header Position: {}", self.header)
        
    }
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
    
    pub fn set_delimiter(&mut self, del: char) -> &mut Self{
        self.delimiter = del;
        self
    }

    pub fn set_header_pos(&mut self, header: u32) -> &mut Self{
        self.header = header;
        self
    }

    pub fn set_encoding(&mut self, new_encoding: &'static Encoding) -> &mut Self{
        self.used_encoding = new_encoding;
        self
    }

    pub fn set_file_type(&mut self, file_type: AcceptedType) -> &mut Self{
        self.file_type = file_type;
        self
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
