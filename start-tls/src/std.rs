use std::io::{Read, Result, Write};

use crate::{Runtime, StartTls, StartTlsExt};

pub struct Blocking;

impl Runtime for Blocking {
    type Context<'a> = ();
    type Output<T> = T;
}

impl<S, T> StartTls<Blocking, S, T>
where
    S: Read + Write,
    T: StartTlsExt<Blocking, S>,
{
    pub fn prepare(mut self) -> Result<()> {
        self.ext.poll(&mut ())
    }
}
