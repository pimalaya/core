//! # TCP
//!
//! This module contains shared TCP code for both server and
//! client.

#[cfg(feature = "tokio")]
use std::{pin::Pin, task::Poll};

#[cfg(feature = "async-std")]
pub use async_std::net::TcpStream;
use futures::{
    io::{BufReader, ReadHalf, WriteHalf},
    AsyncReadExt,
};
#[cfg(feature = "tokio")]
use futures::{ready, AsyncRead, AsyncWrite};

/// The TCP shared configuration between clients and servers.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct TcpConfig {
    /// The TCP host name.
    pub host: String,

    /// The TCP port.
    pub port: u16,
}

pub struct TcpHandler {
    pub reader: BufReader<ReadHalf<TcpStream>>,
    pub writer: WriteHalf<TcpStream>,
}

impl TcpHandler {
    pub fn new(stream: impl Into<TcpStream>) -> Self {
        let (reader, writer) = AsyncReadExt::split(stream.into());
        let reader = BufReader::new(reader);
        Self { reader, writer }
    }
}

#[cfg(feature = "tokio")]
pub struct TcpStream(tokio::net::TcpStream);

#[cfg(feature = "tokio")]
impl TcpStream {
    pub async fn connect<A: tokio::net::ToSocketAddrs>(
        addr: A,
    ) -> tokio::io::Result<tokio::net::TcpStream> {
        tokio::net::TcpStream::connect(addr).await
    }
}

#[cfg(feature = "tokio")]
impl From<tokio::net::TcpStream> for TcpStream {
    fn from(stream: tokio::net::TcpStream) -> Self {
        Self(stream)
    }
}

#[cfg(feature = "tokio")]
impl AsyncRead for TcpStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match ready!(self.0.poll_read_ready(cx)) {
            Err(err) => Poll::Ready(Err(err)),
            Ok(()) => Poll::Ready(self.0.try_read(buf)),
        }
    }
}

#[cfg(feature = "tokio")]
impl AsyncWrite for TcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match ready!(self.0.poll_write_ready(cx)) {
            Err(err) => Poll::Ready(Err(err)),
            Ok(()) => Poll::Ready(self.0.try_write(buf)),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        todo!()
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        todo!()
    }
}
