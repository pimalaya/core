use std::{future::Future, io::Result};

#[cfg(feature = "blocking")]
pub mod blocking;
pub mod imap;

#[cfg(feature = "async")]
pub trait Stoptls<S> {
    fn next(self, stream: S) -> impl Future<Output = Result<S>>;
}
