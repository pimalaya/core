use std::{
    io::Result,
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures::{AsyncRead, AsyncWrite};

use super::{StreamExt, TlsStream};

#[cfg(all(feature = "async-std", feature = "futures-rustls"))]
impl<S: StreamExt> From<futures_rustls::client::TlsStream<S>> for TlsStream<S> {
    fn from(stream: futures_rustls::client::TlsStream<S>) -> Self {
        use tokio_util::compat::FuturesAsyncReadCompatExt;
        Self::FuturesRustls(stream.compat())
    }
}

#[cfg(all(feature = "async-std", feature = "async-native-tls"))]
impl<S: StreamExt> From<async_native_tls::TlsStream<S>> for TlsStream<S> {
    fn from(stream: async_native_tls::TlsStream<S>) -> Self {
        use tokio_util::compat::FuturesAsyncReadCompatExt;
        Self::AsyncNativeTls(stream.compat())
    }
}

impl<S: StreamExt> AsyncRead for TlsStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        match self.get_mut() {
            #[cfg(all(feature = "std", feature = "rustls"))]
            Self::Rustls(stream) => {
                let stream = Pin::new(stream.get_mut());
                let n = ready!(futures::io::AsyncRead::poll_read(stream, cx, buf))?;
                Poll::Ready(Ok(n))
            }
            #[cfg(all(feature = "std", feature = "native-tls"))]
            Self::NativeTls(stream) => {
                let stream = Pin::new(stream.get_mut());
                let n = ready!(futures::io::AsyncRead::poll_read(stream, cx, buf))?;
                Poll::Ready(Ok(n))
            }
            #[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
            Self::TokioRustls(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(all(feature = "tokio", feature = "tokio-native-tls"))]
            Self::TokioNativeTls(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(all(feature = "async-std", feature = "futures-rustls"))]
            Self::FuturesRustls(stream) => Pin::new(stream.get_mut()).poll_read(cx, buf),
            #[cfg(all(feature = "async-std", feature = "async-native-tls"))]
            Self::AsyncNativeTls(stream) => Pin::new(stream.get_mut()).poll_read(cx, buf),
            _ => Poll::Pending,
        }
    }
}

impl<S: StreamExt> AsyncWrite for TlsStream<S> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        match self.get_mut() {
            #[cfg(all(feature = "std", feature = "rustls"))]
            Self::Rustls(stream) => {
                let stream = Pin::new(stream.get_mut());
                futures::io::AsyncWrite::poll_write(stream, cx, buf)
            }
            #[cfg(all(feature = "std", feature = "native-tls"))]
            Self::NativeTls(stream) => {
                let stream = Pin::new(stream.get_mut());
                futures::io::AsyncWrite::poll_write(stream, cx, buf)
            }
            #[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
            Self::TokioRustls(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(all(feature = "tokio", feature = "tokio-native-tls"))]
            Self::TokioNativeTls(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(all(feature = "async-std", feature = "futures-rustls"))]
            Self::FuturesRustls(stream) => Pin::new(stream.get_mut()).poll_write(cx, buf),
            #[cfg(all(feature = "async-std", feature = "async-native-tls"))]
            Self::AsyncNativeTls(stream) => Pin::new(stream.get_mut()).poll_write(cx, buf),
            _ => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self.get_mut() {
            #[cfg(all(feature = "std", feature = "rustls"))]
            Self::Rustls(stream) => {
                let stream = Pin::new(stream.get_mut());
                futures::io::AsyncWrite::poll_flush(stream, cx)
            }
            #[cfg(all(feature = "std", feature = "native-tls"))]
            Self::NativeTls(stream) => {
                let stream = Pin::new(stream.get_mut());
                futures::io::AsyncWrite::poll_flush(stream, cx)
            }
            #[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
            Self::TokioRustls(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(all(feature = "tokio", feature = "tokio-native-tls"))]
            Self::TokioNativeTls(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(all(feature = "async-std", feature = "futures-rustls"))]
            Self::FuturesRustls(stream) => Pin::new(stream.get_mut()).poll_flush(cx),
            #[cfg(all(feature = "async-std", feature = "async-native-tls"))]
            Self::AsyncNativeTls(stream) => Pin::new(stream.get_mut()).poll_flush(cx),
            _ => Poll::Pending,
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self.get_mut() {
            #[cfg(all(feature = "std", feature = "rustls"))]
            Self::Rustls(stream) => {
                let stream = Pin::new(stream.get_mut());
                futures::io::AsyncWrite::poll_close(stream, cx)
            }
            #[cfg(all(feature = "std", feature = "native-tls"))]
            Self::NativeTls(stream) => {
                let stream = Pin::new(stream.get_mut());
                futures::io::AsyncWrite::poll_close(stream, cx)
            }
            #[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
            Self::TokioRustls(stream) => Pin::new(stream).poll_close(cx),
            #[cfg(all(feature = "tokio", feature = "tokio-native-tls"))]
            Self::TokioNativeTls(stream) => Pin::new(stream).poll_close(cx),
            #[cfg(all(feature = "async-std", feature = "futures-rustls"))]
            Self::FuturesRustls(stream) => Pin::new(stream.get_mut()).poll_close(cx),
            #[cfg(all(feature = "async-std", feature = "async-native-tls"))]
            Self::AsyncNativeTls(stream) => Pin::new(stream.get_mut()).poll_close(cx),
            _ => Poll::Pending,
        }
    }
}
