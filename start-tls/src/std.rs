use std::io::{Read, Result, Write};

use crate::{StartTls, StartTlsExt};

impl<S, T> StartTls<S, T, false>
where
    S: Read + Write,
    T: for<'a> StartTlsExt<S, false, Context<'a> = (), Output<()> = Result<()>>,
{
    pub fn prepare(mut self) -> Result<()> {
        self.ext.poll(&mut ())
    }
}
