use std::{any::Any, error, result};

use tokio::task::JoinError;

/// The global any `Result` alias of the library.
///
/// The difference with [`Result`] is that it takes a dynamic error
/// `Box<dyn AnyError>`.
pub type AnyResult<T> = result::Result<T, AnyBoxedError>;

/// The global, dowcastable any `Error` trait of the library.
///
/// This trait is used instead of [`Error`] when an error that is not
/// known at compilation time cannot be placed in a generic due to
/// object-safe trait constraint. The main use case is for backend
/// features.
pub trait AnyError: error::Error + Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
}

impl AnyError for JoinError {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The global any boxed `Error` alias of the module.
pub type AnyBoxedError = Box<dyn AnyError + Send + 'static>;

impl error::Error for AnyBoxedError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.as_ref().source()
    }
}

impl From<JoinError> for AnyBoxedError {
    fn from(err: JoinError) -> Self {
        Box::new(err)
    }
}
