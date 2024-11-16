use std::io::{Read, Result, Write};

use tracing::debug;

use crate::{std::Blocking, StartTlsExt};

use super::ImapStartTls;

impl<S: Read + Write> StartTlsExt<Blocking, S> for ImapStartTls<'_, Blocking, S> {
    fn poll(&mut self, _cx: &mut ()) -> Result<()> {
        if !self.handshake_discarded {
            let n = self.stream.read(&mut self.buf)?;
            let plain = String::from_utf8_lossy(&self.buf[..n]);
            debug!("read and discarded {n} bytes: {plain:?}");
            self.buf.fill(0);
            self.handshake_discarded = true;
        }

        let n = self.stream.write(Self::COMMAND.as_bytes())?;
        debug!("wrote {n} bytes: {:?}", Self::COMMAND);

        let n = self.stream.read(&mut self.buf)?;
        let plain = String::from_utf8_lossy(&self.buf[..n]);
        debug!("read and discarded {n} bytes: {plain:?}");
        self.buf.fill(0);

        self.stream.flush()
    }
}
