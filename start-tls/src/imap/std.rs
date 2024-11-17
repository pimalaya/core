use std::io::{Read, Result, Write};

use crate::blocking;

use super::ImapStartTls;

impl<S: Read + Write> blocking::StartTlsExt<S> for ImapStartTls {
    fn prepare(mut self, stream: &mut S) -> Result<()> {
        if !self.handshake_discarded {
            let count = stream.read(&mut self.read_buffer)?;
            self.post_read(count);
        }

        let count = stream.write(Self::COMMAND.as_bytes())?;
        self.post_write(count);

        let count = stream.read(&mut self.read_buffer)?;
        self.post_read(count);

        stream.flush()
    }
}
