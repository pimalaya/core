use std::{
    io::Result,
    pin::Pin,
    task::{Context, Poll},
};

#[cfg(feature = "async-std")]
use async_std::net::{TcpStream, ToSocketAddrs};
use futures::{AsyncRead, AsyncWrite};

#[cfg(feature = "async-std")]
impl super::TcpStream {
    pub async fn async_std_connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(TcpStream::connect(addr).await?.into())
    }
}

#[cfg(feature = "async-std")]
impl From<TcpStream> for super::TcpStream {
    fn from(stream: TcpStream) -> Self {
        Self::AsyncStd(stream)
    }
}

impl AsyncRead for super::TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        match self.get_mut() {
            #[cfg(feature = "std")]
            Self::Std(stream) => {
                // NOTE: does it block the main thread?
                Poll::Ready(std::io::Read::read(stream, buf))
            }
            #[cfg(feature = "async-std")]
            Self::AsyncStd(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(feature = "tokio")]
            Self::Tokio(stream) => Pin::new(stream).poll_read(cx, buf),
            _ => Poll::Pending,
        }
    }
}

impl AsyncWrite for super::TcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        match self.get_mut() {
            #[cfg(feature = "std")]
            Self::Std(stream) => {
                // NOTE: does it block the main thread?
                Poll::Ready(std::io::Write::write(stream, buf))
            }
            #[cfg(feature = "async-std")]
            Self::AsyncStd(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(feature = "tokio")]
            Self::Tokio(stream) => Pin::new(stream).poll_write(cx, buf),
            _ => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self.get_mut() {
            #[cfg(feature = "std")]
            Self::Std(stream) => {
                // NOTE: does it block the main thread?
                Poll::Ready(std::io::Write::flush(stream))
            }
            #[cfg(feature = "async-std")]
            Self::AsyncStd(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(feature = "tokio")]
            Self::Tokio(stream) => Pin::new(stream).poll_flush(cx),
            _ => Poll::Pending,
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self.get_mut() {
            #[cfg(feature = "std")]
            Self::Std(stream) => {
                // NOTE: does it block the main thread?
                Poll::Ready(stream.shutdown(std::net::Shutdown::Both))
            }
            #[cfg(feature = "async-std")]
            Self::AsyncStd(stream) => Pin::new(stream).poll_close(cx),
            #[cfg(feature = "tokio")]
            Self::Tokio(stream) => Pin::new(stream).poll_close(cx),
            _ => Poll::Pending,
        }
    }
}
