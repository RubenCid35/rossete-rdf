use std::{fmt::Debug, path::PathBuf};

use super::RMLComponent;

pub trait DataSourceIterator: Debug {
    /// Read and parses a block of data from the data source. The result is contained in the
    fn read_block(&mut self) -> miette::Result<()>;
}

#[derive(Debug)]
pub(crate) enum SourceType {
    CSV,
    TSV,
    XML,
    JSON,
    DB,
}

#[derive(Debug)]
pub(crate) enum RefFormulation {
    ROW,
    CSV,
    XPath,
    JSONPath,
}

impl RefFormulation {
    pub fn from_str(formulation: &str) -> Option<Self> {
        match formulation {
            "csv" => Some(Self::CSV),
            "xpath" => Some(Self::XPath),
            "jsonpath" => Some(Self::JSONPath),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct LogicalSource {
    iterator: String,
    source: PathBuf,
    source_type: SourceType,
    reference_formulation: RefFormulation,
}

impl RMLComponent for LogicalSource {}
impl DataSourceIterator for LogicalSource {
    fn read_block(&mut self) -> miette::Result<()> {
        // this wont read. it is temporary.
        Ok(())
    }
}

impl LogicalSource {
    /// Create a new instance of the LogicalSource from its basic fields
    /// Arguments:
    /// * `source`: File path of the data source
    /// * `iterator`: Determines how is the data iterated over. In the case of the CSV, it will be the rows.
    /// * `formulation`: Determines the file formulation for the parsing of elements.
    pub fn new(source: String, iterator: String, formulation: RefFormulation) -> Self {
        let extension = source.split('.').last().unwrap();
        let source_type = match extension {
            "xml" => SourceType::XML,
            "csv" => SourceType::CSV,
            "tsv" => SourceType::TSV,
            "json" => SourceType::JSON,
            _ => unimplemented!("The source type \"{extension}\" is not implemented yet."),
        };

        Self {
            iterator,
            source_type,
            source: PathBuf::from(source),
            reference_formulation: formulation,
        }
    }
}
