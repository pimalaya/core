use std::io::Result;

use futures_util::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::debug;

use crate::StartTlsExt;

use super::SmtpStartTls;

impl<S: AsyncRead + AsyncWrite + Unpin> StartTlsExt<S> for SmtpStartTls {
    async fn prepare(&mut self, stream: &mut S) -> Result<()> {
        if !self.handshake_discarded {
            let n = stream.read(&mut self.read_buffer).await?;
            let plain = String::from_utf8_lossy(&self.read_buffer[..n]);
            debug!("read and discarded {n} bytes: {plain:?}");
            self.read_buffer.fill(0);
        }

        let n = stream.write(Self::COMMAND.as_bytes()).await?;
        debug!("wrote {n} bytes: {:?}", Self::COMMAND);

        let n = stream.read(&mut self.read_buffer).await?;
        let plain = String::from_utf8_lossy(&self.read_buffer[..n]);
        debug!("read and discarded {n} bytes: {plain:?}");
        self.read_buffer.fill(0);

        stream.flush().await
    }
}
