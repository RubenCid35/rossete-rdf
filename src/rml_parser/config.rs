use std::path::PathBuf;

/// Struct that contains all the necesary configuration and file information
/// for the file parsings steps.
pub struct ParseFileConfig {
    /// Path to file location
    pub file_path: PathBuf,
    /// whether to hide or show all the warnings.
    pub silent: bool,
}

impl ParseFileConfig {
    /// Retrieve file path as a string. The path can be used in errors or for display purposes.
    pub fn get_file(&self) -> String {
        self.file_path.to_str().unwrap().to_string()
    }
}

impl std::default::Default for ParseFileConfig {
    fn default() -> Self {
        Self {
            file_path: PathBuf::new(),
            silent: true,
        }
    }
}
