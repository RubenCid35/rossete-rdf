//! # Terms & Literal Representation
//! 
//! This module provides a foundational set of term-type definitions and term generation mechanisms.
//! In an RML mapping, term generators specify how to generate output terms or literals. 
//! For example, when processing a data source like a CSV file, term generators define how to create terms
//! (subjects, predicates, or objects) for each row in the input.
//! 
//! This information is essential for generating output knowledge graphs and ensuring their correct interpretation.
//! By supporting the creation of RDF triples, this module facilitates the building blocks of the output graph, 
//! ensuring consistency and precision.
//! 

use super::RMLComponent;

/// # Different types for node representation.
/// 
/// Each of the terms or nodes can be represented in one of three forms: literal (string), IRI (a encoded url) or
/// term pair. This enum allows the definition of type in the final output of the program.
#[derive(Debug)]
pub(crate) enum TermType {
    /// Representation of a node as an IRI-encoded text, e.g., `<http://example.com>`.
    /// This type of node is commonly used in the subject and object positions of RDF triples. 
    IRI,

    /// Literal text representation. Although the representation is textual, its content may represent a number, date, 
    /// or other data types, often associated with a specific datatype. This is commonly used in the object part of a knowledge triple. 
    /// For example: `"text"^^xsd:string`
    Text,

    /// Compose Term representation. In some output and input formats, the term is defined by a prefix (predefined) and 
    /// the tag of the node. This representation is common in the predicate and some object parts of a knowledge triple. One example is: `ex:mapping`
    Pair,
}

/// Different methods of generating terms or literals in an RML mapping context. 
/// 
/// This enum represents the different methods of generating terms or literals in an RML mapping context.
/// These terms can serve as subjects, predicates, or objects in RDF triples, enabling the creation
/// of knowledge graphs.
#[derive(Debug)]
pub(crate) enum TermGenerators {
    /// Constant Term Generation. In a mapping, there are terms that a preset in the mapping such it is the caso 
    /// of predicate objects. There are other cases like objects with terms. The generator contains the following fields:
    /// 1. Raw Constant (String)
    /// 2. Term Type ([TermType]) with the final representation.
    /// 3. Data Type. Each value has its corresponding data type. By default, each literal is considered a string. 
    ///     This **only applies to literal terms**.
    Constant(String, TermType, Option<String>),

    /// Creates a term from a raw field in the data source. The value from the field is
    /// treated as a string and converted into the specified term type. The generator contains the following fields:
    /// 1. Field Reference as a String. The existence of the field was not checked when generator was created.
    /// 2. Term Type ([TermType]) with the final representation.
    /// 3. Data Type. Each value has its corresponding data type. By default, each literal is assumed ot be a string. 
    ///     This **only applies to literal terms**.
    Reference(String, TermType, Option<String>),

    /// Generates a term by interpolating raw field values into a template string. This allows
    /// for more complex term creation based on patterns or dynamic data from multiple fields.The generator contains the following fields:
    /// 1. Base Template that was formatted to be used in `format!`.
    /// 2. List of referenced fields. Their existence was not checked while parsing.
    /// 3. Term Type ([TermType]) with the final representation.
    TemplateTerm(String, Vec<String>, TermType),

    /// A placeholder variant used to initialize terms or predicates in mappings with split declarations. This has no attached logic for the generation of terms.
    Undeclared,
}

impl TermGenerators {
    /// Generate a template term generator with the extracted fields from a string.
    /// The last argument determines if the final value is an IRI or literal. This
    /// is important for the subject generation.
    ///
    /// Arguments:
    /// * `template`: original string with the template.
    /// * `uri`: whether the output is an URI or a text.
    pub fn template_from_str(template: String, uri: bool) -> Self {
        let ty = if uri { TermType::IRI } else { TermType::Text };
        let mut fields = Vec::new();
        let mut template_text = String::with_capacity(template.len());

        let mut is_field = false;
        let mut field = String::with_capacity(12);

        for c in template.chars() {
            match c {
                '{' if !is_field => {
                    template_text.push(c);
                    is_field = true;
                }
                '}' if is_field => {
                    template_text.push(c);
                    fields.push(field.clone());
                    field.clear();
                    is_field = false;
                }
                _ => {
                    if is_field {
                        field.push(c);
                    } else {
                        template_text.push(c);
                    }
                }
            }
        }

        TermGenerators::TemplateTerm(template_text, fields, ty)
    }
}

impl RMLComponent for TermGenerators {}
