use std::{
    future::{poll_fn, Future},
    io::Result,
    pin::{pin, Pin},
    task::{ready, Context, Poll},
};

use futures_util::{io::Cursor, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, FutureExt};
use tracing::{debug, instrument};

use crate::{ReadBuffer, WriteBuffer};

pub struct BufStream<S> {
    stream: S,
    read_buffer: ReadBuffer,
    write_buffer: WriteBuffer,
}

impl<S> BufStream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            read_buffer: Default::default(),
            write_buffer: Default::default(),
        }
    }

    pub fn set_read_capacity(&mut self, capacity: usize) {
        self.read_buffer.set_capacity(capacity)
    }

    pub fn with_read_capacity(mut self, capacity: usize) -> Self {
        self.read_buffer.set_capacity(capacity);
        self
    }

    pub fn wants_read(&self) -> bool {
        self.read_buffer.wants_read()
    }

    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> BufStream<S> {
    #[instrument(skip_all)]
    pub async fn progress_read(&mut self) -> Result<usize> {
        let slice = &mut self.read_buffer.to_io_slice_mut();
        let count = self.stream.read_vectored(slice).await?;
        self.read_buffer.progress(count)
    }

    #[instrument(skip_all)]
    pub async fn progress_write(&mut self) -> Result<usize> {
        if !self.write_buffer.wants_write() {
            return Ok(0);
        }

        let slices = &mut self.write_buffer.to_io_slices();
        let count = self.stream.write_vectored(slices).await?;
        self.write_buffer.progress(count)
    }

    pub async fn progress(&mut self) -> Result<&[u8]> {
        let count = poll_fn(|cx| {
            match pin!(self.progress_write()).poll_unpin(cx)? {
                Poll::Ready(0) => {
                    debug!("nothing to write");
                }
                Poll::Ready(n) => {
                    debug!("wrote {n} bytes");
                }
                Poll::Pending => {
                    debug!("writing still ongoing");
                }
            }

            match pin!(self.progress_read()).poll_unpin(cx)? {
                Poll::Ready(count) => {
                    debug!("read {count} bytes");
                    Poll::Ready(Result::Ok(count))
                }
                Poll::Pending => {
                    debug!("reading still ongoing");
                    Poll::Pending
                }
            }
        })
        .await?;

        self.stream.flush().await?;

        Ok(&self.read_buffer.as_slice()[..count])
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncRead for BufStream<S> {
    #[instrument(skip_all)]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        if !self.read_buffer.wants_read() {
            return Poll::Ready(Ok(0));
        }

        let this = self.get_mut();
        let mut buf = Cursor::new(buf);
        let write = pin!(buf.write(&this.read_buffer.as_slice()));
        let count = ready!(write.poll(cx))?;
        Poll::Ready(this.read_buffer.sync(count))
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> AsyncWrite for BufStream<S> {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.get_mut().write_buffer.extend(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let this = self.get_mut();
        ready!(pin!(this.progress_write()).poll(cx))?;
        Pin::new(this.get_mut()).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(self.get_mut().get_mut()).poll_close(cx)
    }
}
