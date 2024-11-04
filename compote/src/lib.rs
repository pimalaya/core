#[cfg(feature = "tcp")]
pub mod tcp;
#[cfg(feature = "time")]
pub mod time;
#[cfg(feature = "tls")]
pub mod tls;

#[cfg(feature = "async-std")]
pub trait AsyncStdStreamExt: futures::AsyncRead + futures::AsyncWrite + Unpin {}
#[cfg(feature = "async-std")]
impl<T: futures::AsyncRead + futures::AsyncWrite + Unpin> AsyncStdStreamExt for T {}

#[cfg(feature = "std")]
pub trait StdStreamExt: std::io::Read + std::io::Write + Unpin {}
#[cfg(feature = "std")]
impl<T: std::io::Read + std::io::Write + Unpin> StdStreamExt for T {}

#[cfg(feature = "tokio")]
pub trait TokioStreamExt: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin {}
#[cfg(feature = "tokio")]
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> TokioStreamExt for T {}
