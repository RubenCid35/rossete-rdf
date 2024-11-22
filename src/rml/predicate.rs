use std::collections::HashMap;

use super::RMLComponent;
use super::TermGenerators;

/*
- reference / constant
- join condition
*/

#[derive(Debug)]
pub struct JoinCondition {}

// TODO add doc
#[derive(Debug)]
pub enum PredicateMap {
    ByField(TermGenerators, TermGenerators),
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

/// # Predicate-Object Builder
/// There are cases where the predicate-object map is defined only by references its components.
/// These kind of declarations are common in traductions from YARRML to RML and give some
/// legibility benefits for computers. An example of this behaviour is the following extract from an examples mapping:
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
/// This struct serves as an intermidiate step while parsing this kind of indirect declaration of the components.
/// For its creation, the mapping takes the references and then it can build the corresponding (PredicateMap)[`PredicateMap`] using
/// this information and the remaining detected components.
#[derive(Debug)]
pub struct PredicateBuilder {
    /// References to the predicate declaration. In most instances, this will be defined by a [`TermGenerators`]
    predicate_ref: String,

    /// References to the object-map declaration. In most instances, this will be defined by a [`PredicateMap`]. THis map will be incomplete
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

    // TODO implement + add doc
    pub fn build<R: std::fmt::Debug + RMLComponent>(
        self,
        objects: &mut HashMap<String, Box<R>>,
    ) -> Result<PredicateMap, miette::Error> {
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
