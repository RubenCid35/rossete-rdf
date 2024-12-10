use super::predicate::PredicateMap;
use super::sources::DataSourceIterator;
use super::subject::SubjectMap;
use super::RMLComponent;

#[derive(Debug)]
pub struct Mapping {
    /// mapping id.
    pub id: String,

    /// data logical source. Its kind can vary with the mapping from a CSV, JSON, XML or DB reader.
    logical: Option<Box<dyn DataSourceIterator>>,

    /// subject map. Indicates how each of the resulting pairs are called in the final map.
    subject: Option<SubjectMap>,

    /// predicate list. Each item in the list contains a rule that is used to generate a property to each triple.
    predicates: Vec<PredicateMap>,
}

impl RMLComponent for Mapping {}
