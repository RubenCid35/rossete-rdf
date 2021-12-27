use std::fmt;

pub enum AcceptedType{
    CSV,
    TSV,
    JSON,
    XML,
    Unspecify,
    Other
}
impl fmt::Display for AcceptedType{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self{
            Self::CSV => write!(f, "CSV"),
            Self::TSV => write!(f, "TSV"),
            Self::JSON => write!(f, "JSON"),
            Self::XML => write!(f, "XML"),
            Self::Other => write!(f, "Other"),
            Self::Unspecify => write!(f, "Unspecified")
        }
    }
}
