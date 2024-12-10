//! # Predicate & Triple Generators Representation
//!
//! This module provides with a set of methods for the generation of pairs predicate-object
//! from a data source. In each mapping, a subject requires a set of predicates that describe
//! its properties. There are multiple ways those pairs are created, the two main methods of 
//! creation are: from data source or from other mapping (join condition). 
//! 
//! This module provides a general predicate-object representation that can handle all the possible
//! cases of generation. The other struct is a builder that can be used while parsing a mapping file
//! to generate a predicate struct.

use std::collections::HashMap;

use super::RMLComponent;
use super::TermGenerators;


/// # Predicate-Object Builder
/// 
/// There are cases where the predicate-object map is defined only by reference to its components.
/// These kinds of declarations are common in translations from YARRML to RML and give some
/// legibility benefits for computers. An example of this behaviour is the following extract from an example mapping:
/// ```
/// map:om_001 rml:reference "firstname" ;
///     rr:datatype xsd:string;
///     rdf:type rr:ObjectMap ;
///     rr:termType rr:Literal .
///
/// map:pm_001 rdf:type rr:PredicateMap ;
///     rr:constant ex:name .
///
/// # Predicate Object-Map
/// map:pom_001 rdf:type rr:PredicateObjectMap ;
///     rr:objectMap map:om_001 ;
///     rr:predicateMap map:pm_001.
///
/// ```
/// This struct serves as an intermediate step while parsing this kind of indirect declaration of the components.
/// For its creation, the mapping takes the references and then it can build the corresponding (PredicateMap)[`PredicateMap`] using
/// this information and the remaining detected components.
#[derive(Debug)]
pub struct PredicateBuilder {
    /// References to the predicate declaration. In most instances, this will be defined by a [`TermGenerators`]
    predicate_ref: String,

    /// References to the object-map declaration. In most instances, this will be defined by a [`PredicateMap`]. This map will be incomplete
    /// and the field that corresponds to the predicate will be missing.
    object_ref: String,

    /// Instances of a nested predicate that was defined without indirection. This item corresponds to a [`TermGenerators`].
    predicate_object: Option<Box<dyn RMLComponent>>,

    /// Instances of [`PredicateMap`]. The object may have been created without indirection.
    object_object: Option<Box<dyn RMLComponent>>,
}

impl PredicateBuilder {
    /// Create a new instance of the predicate builder from the references  to its components.
    /// This is just a helper / constructor method.
    pub fn new(
        predicate_ref: String,
        object_ref: String,
        predicate: Option<Box<dyn RMLComponent>>,
        object: Option<Box<dyn RMLComponent>>,
    ) -> Self {
        Self {
            predicate_ref: predicate_ref,
            object_ref: object_ref,
            object_object: object,
            predicate_object: predicate,
        }
    }

    /// Builds a [PredicateMap] from the given components and the rest of instantiated RML components.
    /// This method requires that:
    /// * `predicate_ref` or `predicate` is instantiated and with a value. In  the case of predicate, it must be a [TermGenerators::Constant]
    /// * `object_ref` or `object` is instantiated and with a value. In the case of object, it must be a [PredicateMap]
    /// 
    /// The kind of the output predicate is dependant of the type of object ([PredicateMap]). 
    pub fn build<R: std::fmt::Debug + RMLComponent>(self, objects: &mut HashMap<String, Box<R>> ) -> Result<PredicateMap, miette::Error> {
        todo!()
    }

    /// Determines if the `predicateBuilder` has all the necessary components to be build. This means the `object_object` and `predicate_object`
    /// are instantiate. This can be from indirection and retrival from the pool of components or by recursive construction of the predicateObjectMapping.
    pub fn is_complete(&self) -> bool {
        self.predicate_object.is_some() & self.object_object.is_some()
    }
}

impl RMLComponent for PredicateBuilder {
    fn is_predicate_builder(&self) -> bool {
        true
    }
}


/// # Join Condition Configuration
/// 
/// This struct allows the configuration of how two mapping are connected. In the RML context, 
/// a join-predicate allows references to other objects that are created at the same time. For example,
/// in a map the user defines the buildings and in other there is an entity that is the street. 
#[derive(Debug)]
pub struct JoinCondition {}

/// # Predicate Mappings
/// 
/// Predicate-mappings can be created from data-source (or constant) or from the connection of 2 triples Map.
/// Both kind of mappings require a predicate and object. In the case of the object, the rml mapping needs to 
/// declare the generation method. And in the case of a join, we require to know how can we compare 2 items and determine if 
/// they correspond each other.    
#[derive(Debug)]
pub(crate) enum PredicateMap {
    /// Generate a predicate-object triple from a data source. In this case, the name tupled contains the fields:
    /// 1. predicate. THis is a constant term [TermGenerators::Constant]
    /// 2. object. Determines which fields and the form of the final object. 
    ByField(TermGenerators, TermGenerators),

    /// Generate a predicate-object triple by joining two triples map. 
    /// 1. predicate. THis is a constant term [TermGenerators::Constant]
    /// 2. other map id.
    /// 3. Join Condition. This join condition defines how are the sources matched.
    ByJoin(TermGenerators, String, Option<JoinCondition>),
}

impl PredicateMap {
    /// set predicate term in the predicate map. This is required for maps that have the different components
    /// in different sections of file, for instance, the converted-RML from YARRML.
    ///
    /// This function consumes the element and returns the updated version.
    ///
    /// Arguments:
    /// * predicate (TermGenerator): predicate generator. It needs to be constant.
    pub fn add_predicate(self, predicate: TermGenerators) -> Result<Self, miette::Error> {
        match self {
            PredicateMap::ByField(_, term_generators1) => Ok(PredicateMap::ByField(predicate, term_generators1)),
            PredicateMap::ByJoin(_, other, join_condition) => {
                Ok(PredicateMap::ByJoin(predicate, other, join_condition))
            }
        }
    }
}

impl RMLComponent for PredicateMap {}
