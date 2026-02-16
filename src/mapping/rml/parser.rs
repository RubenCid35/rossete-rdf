use miette::{NamedSource, SourceSpan};

use super::error::RMLParserErrors;

pub fn parse_rml_demo() -> Result<(), RMLParserErrors> {
    Err(RMLParserErrors::FoundInvalidRMLToken {
        found_token: "invalid".to_string(),
        expected_token: None,
        help_message: "Try adding the appropriate token for the position.".to_string(),
        src: NamedSource::new("example.rml", "This is an invalid RML token".to_string()),
        err_span: SourceSpan::new(0.into(), 7),
    })
}
