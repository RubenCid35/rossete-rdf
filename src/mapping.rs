pub mod rml;
pub mod yarrrml;
pub mod error;

use std::collections::HashMap;

/// Trait for mapping parsers.
///
/// The `MappingParser` trait defines the basic interface required for any mapping parser
/// (such as RML or YARRRML parser implementations). This trait allows abstraction over
/// specific mapping file formats, providing unified methods to parse mapping files and
/// access components by ID or all at once.
pub trait MappingParser {
    
    /// Parse the mapping file and return a mapping of component IDs to their data.
    fn parse(&self) -> Result<HashMap<String, String>, error::ParserErrors>;

    /// Retrieve a reference to the component corresponding to the given ID, if present.
    fn get_component(&self, id: &str) -> Option<&String>;
    
    /// Retrieve a reference to the entire map of all components.
    fn get_all_components(&self) -> &HashMap<String, String>;
}