//! Error definitions related to the mapping parsing process.
//!
//! The `ParserErrors` enum contains all errors that can occur during
//! the parsing of mapping files (such as RML or YARRRML). This enables
//! robust error handling and reporting for any code involved in mapping parsing.

use std::fmt;
use std::path::PathBuf;
use miette::{Diagnostic, Report};
use thiserror::Error;

use std::io;
use std::rc;

use crate::mapping::rml::error::RMLParserErrors;

/// Represents all errors that can occur during the mapping parsing process.
///
/// This enum is used to collect any issues related to parsing of mapping files.
/// Each variant corresponds to a specific kind of failure during parsing,
/// such as inability to read the mapping file from disk, or other parsing errors as the crate evolves.
#[derive(Error, Diagnostic, Clone)]
pub enum ParserErrors {
    /// I/O error encountered while reading the mapping files.
    ///
    /// This error is returned when the mapping parser fails to read
    /// a mapping file (e.g., due to file not found, permission denied, etc.).
    #[error("An error happened while reading the mapping file. Error: {1}")]
    #[diagnostic(
        code(rossete::mapping::parse::io::error),
        help("An error happened while reading the mapping file. Error: {1}"),
        url("https://github.com/RubenCid35/rossete-rdf/issues/new")

    )]
    FailedToReadMappingFile(PathBuf, String),

    /// RML-specific parsing errors (e.g. invalid token).
    #[error(transparent)]
    #[diagnostic(transparent)]
    RmlParserError(#[from] RMLParserErrors),
}

impl fmt::Debug for ParserErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FailedToReadMappingFile(path, e) => writeln!(f, "Failed to read mapping file: {}: {}", path.display(), e),
            Self::RmlParserError(e) => writeln!(f, "{}", Report::new(e.clone())),
        }
    }
}

pub(super) fn parse_io_error(file_path: PathBuf, error: io::Error) -> ParserErrors {
    match error.kind() {
        io::ErrorKind::NotFound => ParserErrors::FailedToReadMappingFile(file_path, "File not found or hidden to program.".to_string()),
        io::ErrorKind::PermissionDenied => ParserErrors::FailedToReadMappingFile(file_path, "Permission denied to read the file.".to_string()),
        
        // Invalid file content due to encoding or other issues.
        io::ErrorKind::InvalidData => ParserErrors::FailedToReadMappingFile(file_path, "Invalid file content due to encoding or other issues.".to_string()),

        /// This error will never happen because we are not reading a directory.
        io::ErrorKind::IsADirectory => ParserErrors::FailedToReadMappingFile(file_path, "The path is a directory.".to_string()),

        /// Most of the issues are unhandled and we prefer to panic instead of returning an error.
        _  => unreachable!("Unhandled io error kind: {:?}", error.kind()),
    }

}