use std::fmt;

#[derive(Clone, Copy)]
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
        }else if file.contains("json"){
            AcceptedType::JSON
        }else if file.contains("tsv"){
            AcceptedType::TSV
        }else if file.contains("xml") || file.contains("xpath"){
            AcceptedType::XML
        }else{
            AcceptedType::Other
        }
    }
}