use std::io::{Read, Result, Write};

use tracing::debug;

use crate::StartTlsExt;

use super::SmtpStartTls;

impl<S: Read + Write> StartTlsExt<S, false> for SmtpStartTls<'_, S, false> {
    type Context<'a> = ();
    type Output<T> = Result<T>;

    fn poll(&mut self, _cx: &mut ()) -> Self::Output<()> {
        let n = self.stream.write(Self::COMMAND.as_bytes())?;
        debug!("wrote {n} bytes: {:?}", Self::COMMAND);

        let n = self.stream.read(&mut self.buf)?;
        let plain = String::from_utf8_lossy(&self.buf[..n]);
        debug!("read and discarded {n} bytes: {plain:?}");
        self.buf.fill(0);

        self.stream.flush()
    }
}
