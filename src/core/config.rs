use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct GlobalSettings {

    /// Mappings path. THis path may point to a file or a directory.
    /// If it points to a directory, all files in the directory will be used as mappings.
    mappings_path: PathBuf,
    
    /// Disable warnings. If this is true, no warnings will be logged.
    disable_warnings: bool

}

impl GlobalSettings {

    /// Create a new instance of the global settings.
    pub fn new(mappings_path: PathBuf) -> Self {
        Self { mappings_path, disable_warnings: false }
    }

    /// Disable warnings. This will disable the logging of warnings.
    pub fn disable_warnings(&mut self) {
        self.disable_warnings = true;
    }

}
