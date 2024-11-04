#[cfg(feature = "futures")]
pub mod async_std;
#[cfg(feature = "std")]
pub mod std;
#[cfg(feature = "tokio")]
pub mod tokio;

#[non_exhaustive]
#[derive(Debug, Default)]
pub enum TcpStream {
    #[default]
    #[doc(hidden)]
    None,
    #[cfg(feature = "async-std")]
    AsyncStd(::async_std::net::TcpStream),
    #[cfg(feature = "std")]
    Std(::std::net::TcpStream),
    #[cfg(feature = "tokio")]
    Tokio(tokio_util::compat::Compat<::tokio::net::TcpStream>),
}
