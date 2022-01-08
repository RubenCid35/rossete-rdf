use std::fmt;

#[derive(Clone)]
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
impl fmt::Debug for AcceptedType{
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

impl AcceptedType{
    pub fn from_str(file: &str) -> Self{
        if file.contains("csv"){
            AcceptedType::CSV
        }else if file.contains("JSON"){
            AcceptedType::JSON
        }else if file.contains("TSV"){
            AcceptedType::TSV
        }else if file.contains("XML"){
            AcceptedType::XML
        }else{
            AcceptedType::Other
        }
    }
}