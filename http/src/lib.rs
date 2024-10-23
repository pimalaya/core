#![cfg_attr(docs_rs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

mod error;

use std::sync::Arc;

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

#[derive(Clone, Debug)]
pub struct Client {
    // TODO: pool?
    agent: Arc<Agent>,
}

impl Client {
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
        let agent = Arc::new(config.new_agent());

        Self { agent }
    }

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

#[cfg(feature = "async-std")]
async fn spawn_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Ok(async_std::task::spawn_blocking(f).await)
}

#[cfg(feature = "tokio")]
async fn spawn_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Ok(tokio::task::spawn_blocking(f).await?)
}
