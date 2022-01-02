use std::error::Error;
use std::io;
use std::sync::mpsc;

use crate::error;

#[derive(Debug, Clone)]
pub enum ApplicationErrors{
    // Input and Data Errors
    FileNotFound,
    FilePermissionDenied,
    FileCantRead,
    FileCantWrite,
    NotEnoughMemory,
    ActionInterrumped,
    PrefixActionsInterrumped,
   // Reading Mapping Errors.
    MissingLogicalSource,
    MissingSubjectMap,
    InvalidSourceDataFormat,
    NoInputFieldURISubject,
    MissingKeyPart,
    IncorrectMappingFormat,
    MissingRMlNamespace,
    FailedToTransmitDataBetweenThreads,
    // Other errors
    Miscelaneous
}
// TODO: Implement Custon Error Trait

impl From<io::Error> for ApplicationErrors{
    fn from(error: io::Error) -> Self{
         match error.kind(){
             io::ErrorKind::OutOfMemory => Self::NotEnoughMemory,
             io::ErrorKind::Interrupted => Self::ActionInterrumped,
             io::ErrorKind::PermissionDenied => Self::FilePermissionDenied,
             io::ErrorKind::WriteZero => Self::FileCantWrite,
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
            error!("Something enabled the prefix writting or reading, SOURCe: {:?}", src);
        }
        Self::PrefixActionsInterrumped
    }
}