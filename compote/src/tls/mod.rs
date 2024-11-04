#[cfg(feature = "async-std")]
pub mod async_std;
#[cfg(feature = "std")]
pub mod std;
#[cfg(feature = "tokio")]
pub mod tokio;

use ::std::marker::PhantomData;

#[cfg(feature = "async-std")]
use crate::AsyncStdStreamExt;
#[cfg(feature = "std")]
use crate::StdStreamExt;
#[cfg(feature = "tokio")]
use crate::TokioStreamExt;

#[cfg(not(feature = "std"))]
pub trait StdStreamExt {}
#[cfg(not(feature = "async-std"))]
pub trait AsyncStdStreamExt {}
#[cfg(not(feature = "tokio"))]
pub trait TokioStreamExt {}

pub trait StreamExt: StdStreamExt + AsyncStdStreamExt + TokioStreamExt {}
impl<T: StdStreamExt + AsyncStdStreamExt + TokioStreamExt> StreamExt for T {}

#[non_exhaustive]
#[derive(Debug)]
pub enum TlsStream<S: StreamExt> {
    #[doc(hidden)]
    None(PhantomData<S>),
    #[cfg(all(feature = "std", feature = "rustls"))]
    Rustls(rustls::StreamOwned<rustls::client::ClientConnection, S>),
    #[cfg(all(feature = "std", feature = "native-tls"))]
    NativeTls(native_tls::TlsStream<S>),
    #[cfg(all(feature = "tokio", feature = "tokio-rustls"))]
    TokioRustls(tokio_util::compat::Compat<tokio_rustls::client::TlsStream<S>>),
    #[cfg(all(feature = "tokio", feature = "tokio-native-tls"))]
    TokioNativeTls(tokio_util::compat::Compat<tokio_native_tls::TlsStream<S>>),
    #[cfg(all(feature = "async-std", feature = "futures-rustls"))]
    FuturesRustls(tokio_util::compat::Compat<futures_rustls::client::TlsStream<S>>),
    #[cfg(all(feature = "async-std", feature = "async-native-tls"))]
    AsyncNativeTls(tokio_util::compat::Compat<async_native_tls::TlsStream<S>>),
}
