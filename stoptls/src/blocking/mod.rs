pub mod imap;

use std::io::Result;

pub trait Stoptls<S> {
    fn next(self, stream: S) -> Result<S>;
}
