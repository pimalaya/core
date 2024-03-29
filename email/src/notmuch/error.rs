use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open notmuch database")]
    OpenDatabase(#[source] notmuch::Error),
    #[error("cannot create query for notmuch database")]
    CreatingQueryFailed(#[source] notmuch::Error),
    #[error("cannot query notmuch database")]
    QueryFailed(#[source] notmuch::Error),
    #[error("cannot close notmuch database")]
    ClosingFailed(#[source] notmuch::Error),
}
