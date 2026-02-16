//! Error definitions related to the mapping parsing process.
//!
//! The `ParserErrors` enum contains all errors that can occur during
//! the parsing of mapping files (such as RML or YARRRML). This enables
//! robust error handling and reporting for any code involved in mapping parsing.

use miette::{Diagnostic, SourceSpan, NamedSource, LabeledSpan};
use thiserror::Error;

use std::io;

/// Represents all errors that can occur during the mapping parsing process.
/// 
/// This enum is used to collect any issues related to parsing of mapping files.
/// Each variant corresponds to a specific kind of failure during parsing, 
/// such as inability to read the mapping file from disk, or other parsing errors as the crate evolves.
#[derive(Error, Diagnostic, Debug, Clone)]
pub enum RMLParserErrors {

    /// Invalid mapping token found.
    #[error("Found invalid RML token: {found_token}. Expected: {expected_token:?}")]
    #[diagnostic(
        code(rossete::mapping::rml::parser::invalid_token),
        help("This token is not expected in this position. Please check the mapping file. {help_message}"),
    )]
    FoundInvalidRMLToken{
        /// The token that was found
        found_token: String,

        /// The token that was found
        expected_token: Option<String>,

        #[source_code]
        src: NamedSource<String>,

        help_message: String,

        /// Source span where the error occurred.
        #[label("Token not expected here.")]
        err_span: SourceSpan,

    }

}
