use std::{
    future::poll_fn,
    io::Result,
    pin::Pin,
    task::{Context, Poll},
};

use futures_io::{AsyncRead, AsyncWrite};
use tracing::debug;

use crate::PollStartTls;

use super::ImapStartTls;

impl<S: AsyncRead + AsyncWrite + Unpin> PollStartTls<S, true> for ImapStartTls<'_, S, true> {
    type Output<T> = Poll<Result<T>>;

    fn poll_start_tls(&mut self, cx: Option<&mut Context<'_>>) -> Self::Output<()> {
        let Some(cx) = cx else {
            return Poll::Pending;
        };

        if !self.command_sent {
            match Pin::new(&mut self.stream).poll_write(cx, Self::COMMAND.as_bytes())? {
                Poll::Ready(n) => {
                    debug!("wrote {n} bytes: {:?}", Self::COMMAND);
                    self.command_sent = true;
                }
                Poll::Pending => {
                    debug!("writing still ongoing");
                }
            }
        }

        match Pin::new(&mut self.stream).poll_read(cx, &mut self.buf)? {
            Poll::Ready(n) => {
                let plain = String::from_utf8_lossy(&self.buf[..n]);
                debug!("read then discarded {n} bytes: {plain:?}");
                self.buf.fill(0);
            }
            Poll::Pending => {
                debug!("reading still ongoing");
                return Poll::Pending;
            }
        }

        Pin::new(&mut self.stream).poll_flush(cx)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> ImapStartTls<'_, S, true> {
    pub async fn prepare(&mut self) -> Result<()> {
        poll_fn(|cx| self.poll_start_tls(Some(cx))).await
    }
}
