use std::path::{Path, PathBuf};
use std::collections::HashMap;

use yaml_rust2::{YamlLoader, Yaml, ScanError};


/// YARRRML Mapping file parser. 
/// 
/// This parser is used to parse the YARRRML mapping file and extract the complete define mappings declarations 
/// from it. It supports orphan components that are found in the global definitions.
/// 
pub struct YARRRMLParser {

    /// Source file. This value is stored for error / logging purposes.
    source_file: PathBuf,

    /// Mappping orphan components that are found in the global definitions.
    components: HashMap<String, Yaml>,

}

impl YARRRMLParser {

    /// Create a new empty instance of the parser from a source file path only.
    pub fn new<P: AsRef<Path>>(source_file: P) -> Self {
        Self {
            source_file: source_file.as_ref().to_path_buf(),
            components: HashMap::new(),
        }
    }
}
