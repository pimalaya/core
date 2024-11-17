use std::io::Result;

use futures_util::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::PrepareStartTls;

use super::SmtpStartTls;

impl<S: AsyncRead + AsyncWrite + Unpin> PrepareStartTls<S> for SmtpStartTls {
    async fn prepare(&mut self, stream: &mut S) -> Result<()> {
        if !self.handshake_discarded {
            let count = stream.read(&mut self.read_buffer).await?;
            self.post_read(count);
        }

        let count = stream.write(Self::COMMAND.as_bytes()).await?;
        self.post_write(count);

        let count = stream.read(&mut self.read_buffer).await?;
        self.post_read(count);

        stream.flush().await
    }
}
