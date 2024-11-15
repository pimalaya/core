use std::{
    io::{Read, Result, Write},
    task::Context,
};

use tracing::debug;

use crate::PollStartTls;

use super::ImapStartTls;

impl<S: Read + Write> PollStartTls<S, false> for ImapStartTls<'_, S, false> {
    type Output<T> = Result<T>;

    fn poll_start_tls(&mut self, _cx: Option<&mut Context<'_>>) -> Self::Output<()> {
        let n = self.stream.write(Self::COMMAND.as_bytes())?;
        debug!("wrote {n} bytes: {:?}", Self::COMMAND);

        let n = self.stream.read(&mut self.buf)?;
        let plain = String::from_utf8_lossy(&self.buf[..n]);
        debug!("read and discarded {n} bytes: {plain:?}");
        self.buf.fill(0);

        self.stream.flush()
    }
}

impl<S: Read + Write> ImapStartTls<'_, S, false> {
    pub fn prepare(&mut self) -> Result<()> {
        self.poll_start_tls(None)
    }
}
