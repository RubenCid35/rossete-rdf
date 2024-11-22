use super::RMLComponent;

/// Kind of the Output Term. This represents the display and format type of the term.
#[derive(Debug)]
pub enum TermType {
    /// IRI output. Ths is the case for subjects and other kind of object/subject components. 
    IRI,

    /// Literal Text
    Text,

    /// pair that is composed by a prefix and uri.
    Pair,
}

/// Diferent types of object and term generators. They are based on the diferent object creation methods
/// from the specifications. This terms will correspond with the object or subject in the triple.
#[derive(Debug)]
pub enum TermGenerators {
    /// Generates a constant triple object or subject.
    Constant(String, TermType, Option<String>),

    /// Object creation using a raw field from the data source. The value of the field is added as a string
    Reference(String, TermType, Option<String>),

    /// Object creation using a template string and the value of raw-fields. The values are added as they are.
    /// In contains 2 values: a template string and a list of values.
    TemplateTerm(String, Vec<String>, TermType),

    /// None. THis variant is only usefull to initilize a predicate in a mapping with split declarations.
    /// For instance, when a YARMML map is transfromed to RML.
    Undeclared,
}

impl TermGenerators {
    /// Generate a template term generator with the extracted fields from a string.
    /// This function extracts the information about the fields from the string. The last
    /// argument determines if the final value is an IRI or literal. This is important for the
    /// subject generation.
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
                    }
                    else {
                        template_text.push(c);
                    }
                }
            }
        }

        TermGenerators::TemplateTerm(template_text, fields, ty)
    }
}

impl RMLComponent for TermGenerators {}