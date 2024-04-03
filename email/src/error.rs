use tokio::task::JoinError;

/// The global `Result` alias of the library.
///
/// Refer to the `Error` documentation for an explanation
/// about the choice of using `anyhow` crate on the library level.
pub type Result<T> = std::result::Result<T, Box<dyn EmailError + 'static>>;

/// The global `Error` trait of the library.
///
/// Downcasting should suffice in most cases; since usecases for precise
/// error variant identification in `email-lib` should be rare.
/// While suitable for most libraries, using one error per module in
/// a large library like `email-lib` complicates communication due to
/// differences in errors.
pub trait EmailError: Send + Sync + std::error::Error + std::any::Any {
    fn as_any(&self) -> &dyn std::any::Any;
}

impl EmailError for JoinError {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl From<JoinError> for Box<dyn EmailError> {
    fn from(value: JoinError) -> Self {
        Box::new(value)
    }
}

impl EmailError for process::error::Error {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl From<process::error::Error> for Box<dyn EmailError> {
    fn from(value: process::error::Error) -> Self {
        Box::new(value)
    }
}
