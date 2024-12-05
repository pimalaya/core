use std::io::Result;

#[cfg(feature = "async")]
use futures_util::{io::BufReader, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt};
use tracing::debug;

#[derive(Clone, Debug, Default)]
pub struct ImapStoptls {
    pub(crate) line: String,
    pub(crate) handshake_discarded: bool,
}

impl ImapStoptls {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_handshake_discarded(&mut self, discarded: bool) {
        self.handshake_discarded = discarded;
    }

    pub fn with_handshake_discarded(mut self, discarded: bool) -> Self {
        self.set_handshake_discarded(discarded);
        self
    }

    pub(crate) const COMMAND: &'static str = "A1 STARTTLS\r\n";

    pub(crate) fn discard_handshake_post_hook(&self) {
        debug!("discarded IMAP greeting: {:?}", self.line);
    }

    pub(crate) fn write_starttls_command_post_hook(&self) {
        debug!("wrote IMAP STARTTLS command: {:?}", Self::COMMAND);
    }

    pub(crate) fn discard_line_pre_hook(&mut self) {
        self.line.clear();
    }

    pub(crate) fn discard_line_post_hook(&mut self) {
        debug!("discarded IMAP response: {:?}", self.line);
    }

    pub(crate) fn is_ready_for_tls(&self) -> bool {
        if self.line.starts_with("A1 ") {
            debug!("stream ready for TLS negociation");
            true
        } else {
            false
        }
    }
}

#[cfg(feature = "async")]
impl<S: AsyncRead + AsyncWrite + Unpin> crate::Stoptls<S> for ImapStoptls {
    async fn next(mut self, mut stream: S) -> Result<S> {
        if !self.handshake_discarded {
            let mut reader = BufReader::new(stream);
            reader.read_line(&mut self.line).await?;
            self.discard_handshake_post_hook();
            stream = reader.into_inner();
        }

        stream.write(Self::COMMAND.as_bytes()).await?;
        self.write_starttls_command_post_hook();
        let mut reader = BufReader::new(stream);

        loop {
            self.discard_line_pre_hook();
            reader.read_line(&mut self.line).await?;
            self.discard_line_post_hook();

            if self.is_ready_for_tls() {
                break;
            }
        }

        Ok(reader.into_inner())
    }
}
