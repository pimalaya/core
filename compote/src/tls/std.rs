use std::io::{Error, ErrorKind, Read, Result, Write};

#[cfg(any(feature = "async-std", feature = "tokio"))]
use futures::{executor::block_on, AsyncReadExt, AsyncWriteExt};

use super::{StreamExt, TlsStream};

#[cfg(feature = "rustls")]
impl<S: StreamExt> From<rustls::StreamOwned<rustls::client::ClientConnection, S>> for TlsStream<S> {
    fn from(stream: rustls::StreamOwned<rustls::client::ClientConnection, S>) -> Self {
        Self::Rustls(stream)
    }
}

#[cfg(feature = "native-tls")]
impl<S: StreamExt> From<native_tls::TlsStream<S>> for TlsStream<S> {
    fn from(stream: native_tls::TlsStream<S>) -> Self {
        Self::NativeTls(stream)
    }
}

impl<S: StreamExt> Read for TlsStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            #[cfg(all(feature = "std", feature = "rustls"))]
            Self::Rustls(stream) => stream.read(buf),
            #[cfg(all(feature = "std", feature = "native-tls"))]
            Self::NativeTls(stream) => stream.read(buf),
            #[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
            Self::TokioRustls(stream) => block_on(AsyncReadExt::read(stream, buf)),
            #[cfg(all(feature = "tokio", feature = "tokio-native-tls"))]
            Self::TokioNativeTls(stream) => block_on(AsyncReadExt::read(stream, buf)),
            #[cfg(all(feature = "async-std", feature = "futures-rustls"))]
            Self::FuturesRustls(stream) => block_on(AsyncReadExt::read(stream.get_mut(), buf)),
            #[cfg(all(feature = "async-std", feature = "async-native-tls"))]
            Self::AsyncNativeTls(stream) => block_on(AsyncReadExt::read(stream.get_mut(), buf)),
            _ => Err(Error::new(
                ErrorKind::Unsupported,
                "cannot read from TLS stream",
            )),
        }
    }
}

impl<S: StreamExt> Write for TlsStream<S> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            #[cfg(all(feature = "std", feature = "rustls"))]
            Self::Rustls(stream) => stream.write(buf),
            #[cfg(all(feature = "std", feature = "native-tls"))]
            Self::NativeTls(stream) => stream.write(buf),
            #[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
            Self::TokioRustls(stream) => block_on(AsyncWriteExt::write(stream, buf)),
            #[cfg(all(feature = "tokio", feature = "tokio-native-tls"))]
            Self::TokioNativeTls(stream) => block_on(AsyncWriteExt::write(stream, buf)),
            #[cfg(all(feature = "async-std", feature = "futures-rustls"))]
            Self::FuturesRustls(stream) => block_on(AsyncWriteExt::write(stream.get_mut(), buf)),
            #[cfg(all(feature = "async-std", feature = "async-native-tls"))]
            Self::AsyncNativeTls(stream) => block_on(AsyncWriteExt::write(stream.get_mut(), buf)),
            _ => Err(Error::new(
                ErrorKind::Unsupported,
                "cannot write into TLS stream",
            )),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            #[cfg(all(feature = "std", feature = "rustls"))]
            Self::Rustls(stream) => stream.flush(),
            #[cfg(all(feature = "std", feature = "native-tls"))]
            Self::NativeTls(stream) => stream.flush(),
            #[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
            Self::TokioRustls(stream) => block_on(AsyncWriteExt::flush(stream)),
            #[cfg(all(feature = "tokio", feature = "tokio-native-tls"))]
            Self::TokioNativeTls(stream) => block_on(AsyncWriteExt::flush(stream)),
            #[cfg(all(feature = "async-std", feature = "futures-rustls"))]
            Self::FuturesRustls(stream) => block_on(AsyncWriteExt::flush(stream.get_mut())),
            #[cfg(all(feature = "async-std", feature = "async-native-tls"))]
            Self::AsyncNativeTls(stream) => block_on(AsyncWriteExt::flush(stream.get_mut())),
            _ => Err(Error::new(
                ErrorKind::Unsupported,
                "cannot flush TLS stream",
            )),
        }
    }
}
