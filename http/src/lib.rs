#![cfg_attr(docs_rs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

mod error;

use std::sync::Arc;

use ureq::{
    config::Config,
    tls::{RootCerts, TlsConfig, TlsProvider},
    Agent, AsSendBody,
};
pub use ureq::{
    http::{Response, StatusCode, Uri},
    Body,
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
    config: Config,
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

        Self { config, agent }
    }

    pub async fn get<U>(&self, uri: U) -> Result<Response<Body>>
    where
        U: Into<Uri> + Clone,
    {
        let req = self.agent.get(uri.clone());
        let res = spawn_blocking(move || req.call())
            .await?
            .map_err(|err| Error::SendGetRequestError(err, uri.into()))?;

        Ok(res)
    }

    pub async fn post<U, B>(&self, uri: U, body: B) -> Result<Response<Body>>
    where
        U: Into<Uri> + Clone,
        B: AsSendBody + Send + 'static,
    {
        let req = self.agent.post(uri.clone());
        let res = spawn_blocking(move || req.send(body))
            .await?
            .map_err(|err| Error::SendPostRequestError(err, uri.into()))?;

        Ok(res)
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
