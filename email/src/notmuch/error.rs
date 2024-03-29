use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open notmuch database")]
    OpenNotmuchDatabase(#[source] notmuch::Error),
    #[error("cannot create query for notmuch database")]
    CreatingQueryNotmuchFailed(#[source] notmuch::Error),
    #[error("cannot query notmuch database")]
    QueryNotmuchFailed(#[source] notmuch::Error),
    #[error("cannot close notmuch database")]
    ClosingNotmuchFailed(#[source] notmuch::Error),
}
