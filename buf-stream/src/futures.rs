use std::{
    future::Future,
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
    async fn progress_read(&mut self) -> Result<usize> {
        let slice = &mut self.read_buffer.to_io_slice_mut();
        let count = self.stream.read_vectored(slice).await?;
        self.read_buffer.progress(count)
    }

    #[instrument(skip_all)]
    async fn progress_write(&mut self) -> Result<usize> {
        if self.write_buffer.wants_write() {
            let slices = &mut self.write_buffer.to_io_slices();
            let count = self.stream.write_vectored(slices).await?;
            self.write_buffer.progress(count)
        } else {
            Ok(0)
        }
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

        match pin!(this.progress_write()).poll_unpin(cx)? {
            Poll::Ready(count) => {
                debug!("wrote {count} bytes");
            }
            Poll::Pending => {
                debug!("writing still ongoing");
            }
        }

        match pin!(this.progress_read()).poll_unpin(cx)? {
            Poll::Ready(count) => {
                debug!("read {count} bytes");
            }
            Poll::Pending => {
                debug!("reading still ongoing");
                return Poll::Pending;
            }
        };

        ready!(Pin::new(this.get_mut()).poll_flush(cx))?;
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(self.get_mut().get_mut()).poll_close(cx)
    }
}
