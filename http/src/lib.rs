#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

mod error;

pub use ureq;
use ureq::{
    config::Config,
    http::Response,
    tls::{RootCerts, TlsConfig, TlsProvider},
    Agent, Body,
};

#[doc(inline)]
pub use crate::error::{Error, Result};

#[cfg(any(
    all(feature = "tokio", feature = "async-std"),
    not(any(feature = "tokio", feature = "async-std"))
))]
compile_error!("Either feature `tokio` or `async-std` must be enabled for this crate.");

#[cfg(any(
    all(feature = "rustls", feature = "native-tls"),
    not(any(feature = "rustls", feature = "native-tls"))
))]
compile_error!("Either feature `rustls` or `native-tls` must be enabled for this crate.");

/// The HTTP client structure.
///
/// This structure wraps a HTTP agent, which is used by the
/// [`Client::send`] function.
#[derive(Clone, Debug)]
pub struct Client {
    /// The HTTP agent used to perform calls.
    agent: Agent,
}

impl Client {
    /// Creates a new HTTP client with sane defaults.
    pub fn new() -> Self {
        let tls = TlsConfig::builder()
            .root_certs(RootCerts::PlatformVerifier)
            .provider(
                #[cfg(feature = "native-tls")]
                TlsProvider::NativeTls,
                #[cfg(feature = "rustls")]
                TlsProvider::Rustls,
            );

        let config = Config::builder().tls_config(tls.build()).build();
        let agent = config.new_agent();

        Self { agent }
    }

    /// Sends a request.
    ///
    /// This function takes a callback that tells how the request
    /// looks like. It takes a reference to the inner HTTP agent as
    /// parameter.
    pub async fn send(
        &self,
        f: impl FnOnce(&Agent) -> std::result::Result<Response<Body>, ureq::Error> + Send + 'static,
    ) -> Result<Response<Body>> {
        let agent = self.agent.clone();

        spawn_blocking(move || f(&agent))
            .await?
            .map_err(Error::SendRequestError)
    }
}

/// Spawns a blocking task using [`async_std`].
#[cfg(feature = "async-std")]
async fn spawn_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Ok(async_std::task::spawn_blocking(f).await)
}

/// Spawns a blocking task using [`tokio`].
#[cfg(feature = "tokio")]
async fn spawn_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Ok(tokio::task::spawn_blocking(f).await?)
}
