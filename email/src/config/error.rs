use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get configuration of account {0}")]
    GetAccountConfigNotFoundError(String),
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
