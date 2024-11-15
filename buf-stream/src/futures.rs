use std::{future::poll_fn, io::Result, pin::pin, task::Poll};

use futures::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    FutureExt,
};
use tracing::{debug, instrument, trace};

pub const ASYNC: bool = true;
pub type BufStream<S> = crate::BufStream<S, ASYNC>;

impl<S: AsyncRead + Unpin> BufStream<S> {
    #[instrument(skip_all)]
    pub async fn progress_read(&mut self) -> Result<usize> {
        let buf = &mut self.read_buf;
        let read_slice = &mut Self::read_slice(buf);
        let byte_count = self.stream.read_vectored(read_slice).await?;
        let byte_count = Self::validate_byte_count(byte_count)?;

        trace!(data = ?buf[..byte_count], "read");

        Ok(byte_count)
    }
}

impl<S: AsyncWrite + Unpin> BufStream<S> {
    #[instrument(skip_all)]
    pub async fn progress_write(&mut self) -> Result<usize> {
        let mut total_byte_count = 0;

        while self.needs_write() {
            let buf = &mut self.write_buf;
            let write_slices = &mut Self::write_slices(buf);
            let byte_count = self.stream.write_vectored(write_slices).await?;

            let bytes = self.write_buf.drain(..byte_count);
            trace!(data = ?bytes, "write");

            drop(bytes);

            total_byte_count += Self::validate_byte_count(byte_count)?;
        }

        Ok(total_byte_count)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> BufStream<S> {
    #[instrument(skip_all)]
    pub async fn progress(&mut self) -> Result<&[u8]> {
        let fut = poll_fn(|cx| {
            if self.needs_write() {
                let poll = pin!(self.progress_write()).poll_unpin(cx);
                debug!(?poll, "writing bytes");

                if let Poll::Ready(Err(err)) = poll {
                    return Poll::Ready(Err(err));
                }
            }

            let poll = pin!(self.progress_read()).poll_unpin(cx);
            debug!(?poll, "reading bytes");

            if let Poll::Ready(res) = poll {
                return Poll::Ready(res);
            }

            Poll::Pending
        });

        let byte_count = fut.await?;

        Ok(&self.read_buf[..byte_count])
    }
}
