//! # Account HTTP discovery
//!
//! This module contains everything needed to discover account using
//! HTTP requests.

use hyper::{body::to_bytes, client::HttpConnector, Client, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};

use super::config::AutoConfig;
#[doc(inline)]
pub use super::{Error, Result};
use crate::trace;

/// Simple HTTP client using rustls connector.
pub struct HttpClient {
    client: Client<HttpsConnector<HttpConnector>>,
}

impl HttpClient {
    /// Create a new HTTP client using defaults.
    pub fn new() -> Self {
        let client = Client::builder().build(
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_or_http()
                .enable_http1()
                .build(),
        );

        Self { client }
    }

    /// Send a GET request to the given URI and try to parse response
    /// as autoconfig.
    pub async fn get_config(&self, uri: Uri) -> Result<AutoConfig> {
        let res = self
            .client
            .get(uri.clone())
            .await
            .map_err(|e| Error::GetConnectionAutoConfigError(uri.clone(), e))?;

        let status = res.status();
        let body = to_bytes(res.into_body())
            .await
            .map_err(|e| Error::GetConnectionAutoConfigError(uri.clone(), e))?;

        // If we got an error response we return an error
        if !status.is_success() {
            trace!("{}", String::from_utf8_lossy(&body));
            return Err(Error::GetAutoConfigError(uri.clone(), status).into());
        }

        let config = serde_xml_rs::from_reader(body.as_ref())
            .map_err(|e| Error::SerdeXmlFailedForAutoConfig(uri, e))?;
        Ok(config)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
