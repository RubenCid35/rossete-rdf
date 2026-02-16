#![allow(dead_code)] // allow dead-code while developing
#![allow(unused)] // allow unused while developing

pub mod core;
pub mod mapping;

mod error;

// re-export structs and errors for application usage.
pub use core::config::GlobalSettings;
pub use error::MaterializerError;
pub use mapping::error::ParserErrors;
pub use mapping::rml::error::RMLParserErrors;



