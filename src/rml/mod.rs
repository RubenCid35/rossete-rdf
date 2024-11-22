//!
//! # RML representation for the program execution.
//!
//! This module contains all the required abstractions and components that are necessary to
//! build the future data reader and rdf writers along side index and iteration optiomizations.
//! THis module will contain the general reprentation of RML Mapping with the most crucial of all the components.

pub mod common;
pub mod map;
pub mod predicate;
pub mod sources;
pub mod subject;

use common::TermGenerators;
pub use map::Mapping;

/// General Trait for a RML Component.
///
/// The trait does not have any functionality attached so i can only be used
/// for box and type managment while parsing. This trait will be used in all
/// the components from logical sources to mapping object itself.
pub trait RMLComponent: std::fmt::Debug {
    /// Method used to determine if the *RMLComponent* is predicateBuilder or not. There are 
    /// some intermidiate builders that are only defined using references to other components.
    /// For example, this mapping `PredicateObjectMap`: 
    /// ```
    /// map:pom_001 rdf:type rr:PredicateObjectMap ;
    ///     rr:objectMap map:om_001 ;
    ///     rr:predicateMap map:pm_001 .
    /// ```
    fn is_predicate_builder(&self) -> bool { false }
}
