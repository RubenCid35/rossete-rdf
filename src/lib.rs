
mod errors;

pub type ResultApp<T> = Result<T, errors::ApplicationErrors>;

pub mod mappings;
pub mod logging;

pub use logging::*;