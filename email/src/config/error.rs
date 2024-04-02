use thiserror::Error;
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get configuration of account {0}")]
    GetAccountConfigNotFoundError(String),
}
