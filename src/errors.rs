#![allow(dead_code)]

use std::any::Any;
use std::error::Error;
use std::io;
use std::sync::mpsc;

use crate::error;
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
    IncorrectXMLFile,

    // Database Errors
    CantOpenDatabase,
    DataBaseDidntReceivedData,
    FailTowriteInDataBase,
    FailToReadInDataBase,
    MissingFieldInData,

    // Reading Mapping Errors.
    PrefixActionsInterrumped,
    MissingLogicalSource,
    MissingSubjectMap,
    InvalidSourceDataFormat, // Maybe it will be eliminated
    NoInputFieldURISubject,
    ComponentInIncorrectLocation,
    IncorrectMappingFormat,
    MissingRMlNamespace, // Future use
    // Other errors
    FailedToTransmitDataBetweenThreads,
    NotEnoughMemory,
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
        Self::FailedToTransmitDataBetweenThreads
    }
}

impl<T> From<std::sync::PoisonError<T>> for ApplicationErrors{
    fn from(error: std::sync::PoisonError<T>) -> Self {
        if let Some(src) = error.source(){
            error!("Something enabled the prefix writting or reading, SOURCE: {:?}", src);
        }
        Self::PrefixActionsInterrumped
    }
}

impl From<Box<dyn Any + Send>> for ApplicationErrors{
    fn from(_: Box<dyn Any + Send>) -> Self {
        Self::FailedToTransmitDataBetweenThreads
    }
}

impl From<sqlite::Error> for ApplicationErrors{
    fn from(error: sqlite::Error) -> Self {
        error!("{:?}", error);
        Self::CantOpenDatabase
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