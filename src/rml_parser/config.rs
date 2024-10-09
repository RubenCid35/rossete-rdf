use std::path::PathBuf;

/// Struct that contains all the necesary configuration and file information
/// for the file parsings steps.
pub struct ParseFileConfig {
    /// Path to file location
    pub file_path: PathBuf,
    pub silent: bool
}

impl ParseFileConfig {
    pub fn get_file(&self) -> String {
        self.file_path.to_str().unwrap().to_string()
    }
}
