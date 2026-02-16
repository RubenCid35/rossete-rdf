//! Error handling module for the entire application.
//!
//! This module provides unified error types that are used throughout the application,
//! enabling consistent and robust error handling. All application-level errors should be
//! represented by the [`MaterializerError`] enum, which serves as a common interface for
//! error propagation and user-facing diagnostics.
//!
//! By forwarding underlying error sources (such as IO errors and mapping parser errors),
//! [`MaterializerError`] enables centralized error reporting and a single point for integrating
//! with user error displays (such as "show error" interfaces).

use std::fmt;
use miette::Diagnostic;
use thiserror::Error;

use crate::mapping::error::ParserErrors;
use crate::mapping::rml::error::RMLParserErrors;

/// Central error type for the Materializer application.
///
/// This enum encapsulates all common errors generated throughout the application.
/// It forwards relevant underlying error types, allowing a uniform approach for error
/// management, reporting, and integration with frontends or "show error" displays.
#[derive(Error, Diagnostic, Debug)]
pub enum MaterializerError {

    /// Represents errors that occur during the mapping parsing process.
    ///
    /// This variant transparently wraps [`ParserErrors`] for detailed parser diagnostics.
    #[error(transparent)]
    #[diagnostic(transparent)]
    MappingParserError(#[from] ParserErrors),
}

impl From<RMLParserErrors> for MaterializerError {

    #[inline]
    fn from(e: RMLParserErrors) -> Self {
        MaterializerError::MappingParserError(ParserErrors::from(e))
    }
}