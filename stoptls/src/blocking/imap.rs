use std::io::{BufRead, BufReader, Read, Result, Write};

use crate::imap::ImapStoptls;

use super::Stoptls;

impl<S: Read + Write> Stoptls<S> for ImapStoptls {
    fn next(mut self, mut stream: S) -> Result<S> {
        if !self.handshake_discarded {
            let mut reader = BufReader::new(stream);
            reader.read_line(&mut self.line)?;
            self.discard_handshake_post_hook();
            stream = reader.into_inner();
        };

        stream.write(Self::COMMAND.as_bytes())?;
        self.write_starttls_command_post_hook();
        let mut stream = BufReader::new(stream);

        loop {
            self.discard_line_pre_hook();
            stream.read_line(&mut self.line)?;
            self.discard_line_post_hook();

            if self.is_ready_for_tls() {
                break;
            }
        }

        Ok(stream.into_inner())
    }
}
