use super::RMLComponent;
use super::TermGenerators;

#[derive(Debug)]
pub struct SubjectMap {
    /// Method used to generate the uri from the data source item
    subject: TermGenerators,

    /// Associated type of a object defined by the subject. The value is supposed to be [TermGenerators::Constant]
    r#type: Option<TermGenerators>,
}

impl SubjectMap {
    /// Create a new subject map from a template uri generator. This can be constant.
    pub fn new(template: TermGenerators) -> Self {
        Self { subject: template, r#type: None }
    }

    /// Add information about the type associated to a subject.
    pub fn add_type(&mut self, ty: TermGenerators) {
        if !matches!(ty, TermGenerators::Constant(_, _, _)) {
            panic!("The associated term type of the type triple of a subject must be a constant, this is: `rdf:type` or `a`")
        }
        self.r#type = Some(ty);
    }
}

impl RMLComponent for SubjectMap {}
