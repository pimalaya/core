use ::std::{future::Future, io::Result};

#[cfg(feature = "blocking")]
pub mod blocking;
pub mod imap;
pub mod smtp;

#[cfg(feature = "async")]
pub trait StartTlsExt<S> {
    fn prepare(&mut self, stream: &mut S) -> impl Future<Output = Result<()>>;
}
