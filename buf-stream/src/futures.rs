use std::{
    future::Future,
    io::Result,
    pin::{pin, Pin},
    task::{ready, Context, Poll},
};

use futures::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Cursor},
    FutureExt,
};
use tracing::{debug, instrument, trace};

pub const ASYNC: bool = true;
pub type BufStream<S> = crate::BufStream<S, ASYNC>;

impl<S: AsyncRead + AsyncWrite + Unpin> BufStream<S> {
    #[instrument(skip_all)]
    async fn progress_read(&mut self) -> Result<usize> {
        let read_slice = &mut Self::read_slice(&mut self.read_buffer);
        let count = self.stream.read_vectored(read_slice).await?;
        Self::check_for_eof(count)?;

        let bytes = &self.read_buffer[..count];
        trace!(?bytes, len = count, "read bytes");
        Ok(count)
    }

    #[instrument(skip_all)]
    async fn progress_write(&mut self) -> Result<usize> {
        let mut total_count = 0;

        while self.wants_write() {
            let write_slices = &mut Self::write_slices(&mut self.write_buffer);
            let count = self.stream.write_vectored(write_slices).await?;
            total_count += Self::check_for_eof(count)?;

            let bytes = self.write_buffer.drain(..count);
            trace!(?bytes, len = count, "wrote bytes");
            drop(bytes)
        }

        Ok(total_count)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for BufStream<S> {
    #[instrument(skip_all)]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        let this = self.get_mut();

        let mut buf = Cursor::new(buf);
        let write = pin!(buf.write(&this.read_buffer[..this.read_cursor]));
        let count = ready!(write.poll(cx))?;
        Self::check_for_eof(count)?;

        this.fill_read_buffer(count);
        Poll::Ready(Ok(count))
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for BufStream<S> {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.get_mut().write_buffer.extend(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let this = self.get_mut();

        match pin!(this.progress_write()).poll_unpin(cx)? {
            Poll::Ready(count) => {
                debug!("wrote {count} bytes");
            }
            Poll::Pending => {
                debug!("writing still ongoing");
            }
        }

        let read_cursor = match pin!(this.progress_read()).poll_unpin(cx)? {
            Poll::Ready(count) => {
                debug!("read {count} bytes");
                count
            }
            Poll::Pending => {
                debug!("reading still ongoing");
                return Poll::Pending;
            }
        };

        ready!(Pin::new(this.get_mut()).poll_flush(cx))?;
        this.read_cursor = read_cursor;
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(self.get_mut().get_mut()).poll_close(cx)
    }
}
