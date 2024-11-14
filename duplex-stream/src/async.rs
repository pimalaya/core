use std::{
    future::poll_fn,
    io::Result,
    pin::{pin, Pin},
    task::{ready, Context, Poll},
};

use futures::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    FutureExt,
};
use tracing::{debug, instrument, trace};

use crate::escape_byte_string;

pub const ASYNC: bool = true;
pub type DuplexStream<S> = crate::DuplexStream<S, ASYNC>;

impl<S: AsyncRead + AsyncWrite + Unpin> DuplexStream<S> {
    pub async fn progress_read(&mut self) -> Result<usize> {
        let buf = &mut self.read_buffer;

        let byte_count = self.stream.read(buf).await?;
        let byte_count = Self::validate_byte_count(byte_count)?;

        trace!(data = escape_byte_string(&buf[..byte_count]), "io/read/raw");

        Ok(byte_count)
    }

    pub async fn progress_write(&mut self) -> Result<usize> {
        let mut total_byte_count = 0;

        while self.needs_write() {
            let ref write_slices = Self::write_slices(&mut self.write_buffer);

            let byte_count = self.stream.write_vectored(write_slices).await?;

            let bytes = self
                .write_buffer
                .range(..byte_count)
                .cloned()
                .collect::<Vec<_>>();

            trace!(data = escape_byte_string(bytes), "io/write/raw");

            // Drop written bytes
            drop(self.write_buffer.drain(..byte_count));

            total_byte_count += Self::validate_byte_count(byte_count)?;
        }

        Ok(total_byte_count)
    }

    #[instrument(skip_all, target = "duplex-stream::async::buffered")]
    pub async fn progress(&mut self) -> Result<&[u8]> {
        let fut = poll_fn(|cx| {
            if self.needs_write() {
                let poll = pin!(self.progress_write()).poll_unpin(cx);
                debug!(?poll, "write");

                if let Poll::Ready(Err(err)) = poll {
                    return Poll::Ready(Err(err));
                }
            }

            let poll = pin!(self.progress_read()).poll_unpin(cx);
            debug!(?poll, "read");

            if let Poll::Ready(res) = poll {
                return Poll::Ready(res);
            }

            Poll::Pending
        });

        let byte_count = fut.await?;

        Ok(&self.read_buffer[..byte_count])
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for DuplexStream<S> {
    #[instrument(skip_all, target = "duplex-stream::async")]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let stream = self.get_mut().get_mut();
        let byte_count = ready!(Pin::new(stream).poll_read(cx, buf))?;
        Poll::Ready(Self::validate_byte_count(byte_count))
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for DuplexStream<S> {
    #[instrument(skip_all, target = "duplex-stream::async")]
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        let stream = self.get_mut().get_mut();
        let byte_count = ready!(Pin::new(stream).poll_write(cx, buf))?;
        Poll::Ready(Self::validate_byte_count(byte_count))
    }

    #[instrument(skip_all, target = "duplex-stream::async")]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let stream = self.get_mut().get_mut();
        Pin::new(stream).poll_flush(cx)
    }

    #[instrument(skip_all, target = "duplex-stream::async")]
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let stream = self.get_mut().get_mut();
        Pin::new(stream).poll_close(cx)
    }
}
