
use std::any::Any;
use std::io;
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ApplicationErrors{
    // Input and Data Errors
    FileNotFound,
    FilePermissionDenied,
    FileCantWrite,
    ActionInterrumped,
    FailToParseCSVData,
    
    // Configuration Errors
    MissingFilePathInConfiguration,
    IncorrectFieldType,
    InvalidDataEntry,
    IncorrectJsonFile,
    IncorrectJsonPath,
    IncorrectXMLFile,
    IncorrectXPath,

    // Database Errors
    CantOpenDatabase,
    DataBaseDidntReceivedData,
    FailedToInteractWithDB,
    MissingFieldInData,

    // Reading Mapping Errors.
    MissingLogicalSource,
    MissingSubjectMap,
    NoInputFieldURISubject,
    ComponentInIncorrectLocation,
    IncorrectMappingFormat,
    MissingClosingBracket,
    MappingNotFound,
     
    // RDF Creations
    FAiledToCreateRDF,

    // Other errors
    FailedToTransmitDataBetweenThreads,
    FailedToReceiveDataBetweenThreads,
    NotEnoughMemory,
    SyncActionUnable,
    Miscelaneous
}

impl From<io::Error> for ApplicationErrors{
    fn from(error: io::Error) -> Self{
         match error.kind(){
             io::ErrorKind::OutOfMemory => Self::NotEnoughMemory,
             io::ErrorKind::Interrupted => Self::ActionInterrumped,
             io::ErrorKind::PermissionDenied => Self::FilePermissionDenied,
             io::ErrorKind::WriteZero => Self::FileCantWrite,
             io::ErrorKind::NotFound => Self::FileNotFound,
             _ => {
                crate::error!("ERROR CODE: {:?}", error);
                Self::Miscelaneous
             }
         } 
    }
}

impl<T> From<mpsc::SendError<T>> for ApplicationErrors{
    fn from(_: mpsc::SendError<T>) -> Self {
        Self::FailedToTransmitDataBetweenThreads
    }
}
impl From<mpsc::RecvError> for ApplicationErrors{
    fn from(_: mpsc::RecvError) -> Self {
        Self::FailedToReceiveDataBetweenThreads
    }
}

impl<T> From<std::sync::PoisonError<T>> for ApplicationErrors{
    fn from(_: std::sync::PoisonError<T>) -> Self {
        Self::SyncActionUnable
    }
}

impl From<Box<dyn Any + Send>> for ApplicationErrors{
    fn from(_: Box<dyn Any + Send>) -> Self {
        Self::Miscelaneous
    }
}

impl From<rusqlite::Error> for ApplicationErrors{
    fn from(error: rusqlite::Error) -> Self{
        match error{
            rusqlite::Error::SqliteFailure(error, data) => {
                if data.is_some(){
                    crate::error!("ERROR DB -> REASON: {}", data.unwrap());
                }
                crate::error!("FFI ERROR: {:?}", error);
                Self::FailedToInteractWithDB
            }
            rusqlite::Error::InvalidColumnIndex(..) => Self::MissingFieldInData,
            rusqlite::Error::InvalidQuery => Self::InvalidDataEntry,
            _ => {
                crate::error!("ERROR DB: {:?}", error);    
                Self::CantOpenDatabase
            }
        }
    }
}

impl From<csv::Error> for ApplicationErrors{
    fn from(_: csv::Error) -> Self {
        Self::FailToParseCSVData
    }
}

impl From<sxd_document::parser::Error> for ApplicationErrors{
    fn from(_: sxd_document::parser::Error) -> Self{
        Self::IncorrectXMLFile
    }
}

impl From<sxd_xpath::Error> for ApplicationErrors{
    fn from(_: sxd_xpath::Error) -> Self{
        Self::IncorrectXPath
    }
}

impl From<serde_json::Error> for ApplicationErrors{
    fn from(_ : serde_json::Error) -> Self{
        Self::IncorrectJsonFile
    }
}
impl From<jsonpath_lib::JsonPathError> for ApplicationErrors{
    fn from(_ : jsonpath_lib::JsonPathError) -> Self{
        Self::IncorrectJsonPath
    }
}

