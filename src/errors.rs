#![allow(dead_code)]

use std::any::Any;
use std::io;
use std::sync::mpsc;

use roxmltree as xml;

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
    InvalidSourceDataFormat, // Maybe it will be eliminated
    NoInputFieldURISubject,
    ComponentInIncorrectLocation,
    IncorrectMappingFormat,
    MissingRMlNamespace, // Future use
    
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
             _ => Self::Miscelaneous
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
            rusqlite::Error::SqliteFailure(..) => Self::CantOpenDatabase,
            rusqlite::Error::InvalidColumnIndex(..) => Self::MissingFieldInData,
            rusqlite::Error::InvalidQuery => Self::InvalidDataEntry,
            _ => Self::DataBaseDidntReceivedData
        }
    }
}

impl From<csv::Error> for ApplicationErrors{
    fn from(_: csv::Error) -> Self {
        Self::FailToParseCSVData
    }
}

impl From<xml::Error> for ApplicationErrors{
    fn from(_: xml::Error) -> Self{
        Self::IncorrectXMLFile
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

