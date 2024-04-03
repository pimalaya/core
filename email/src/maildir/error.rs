#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot expand path: {0}")]
    ExpandPathFailed(#[from] shellexpand_utils::error::Error),
    #[error("maildir checkup failed: {0}")]
    CheckingUpMaildirFailed(#[source] maildirpp::Error),
}

impl crate::EmailError for Error {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl From<Error> for Box<dyn crate::EmailError> {
    fn from(value: Error) -> Self {
        Box::new(value)
    }
}
