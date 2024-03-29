#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot expand path: {0}")]
    ExpandPathFailed(#[from] shellexpand_utils::error::Error),
    #[error("maildir checkup failed: {0}")]
    CheckingUpMaildirFailed(#[source] maildirpp::Error),
}
