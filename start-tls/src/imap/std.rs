use std::io::{Read, Result, Write};

use tracing::debug;

use crate::blocking;

use super::ImapStartTls;

impl<S: Read + Write> blocking::StartTlsExt<S> for ImapStartTls {
    fn prepare(mut self, stream: &mut S) -> Result<()> {
        if !self.handshake_discarded {
            let n = stream.read(&mut self.read_buffer)?;
            let plain = String::from_utf8_lossy(&self.read_buffer[..n]);
            debug!("read and discarded {n} bytes: {plain:?}");
            self.read_buffer.fill(0);
        }

        let n = stream.write(Self::COMMAND.as_bytes())?;
        debug!("wrote {n} bytes: {:?}", Self::COMMAND);

        let n = stream.read(&mut self.read_buffer)?;
        let plain = String::from_utf8_lossy(&self.read_buffer[..n]);
        debug!("read and discarded {n} bytes: {plain:?}");
        self.read_buffer.fill(0);

        stream.flush()
    }
}
