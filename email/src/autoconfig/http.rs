//! # Account HTTP discovery
//!
//! This module contains everything needed to discover account using
//! HTTP requests.

use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};

use super::config::AutoConfig;
#[doc(inline)]
pub use super::{Error, Result};
use crate::trace;

/// Simple HTTP client using rustls connector.
pub struct HttpClient {
    client: Client<HttpsConnector<HttpConnector>, Full<Bytes>>,
}

impl HttpClient {
    /// Create a new HTTP client using defaults.
    pub fn new() -> Result<Self> {
        let conn = HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(Error::CreateHttpConnectorError)?
            .https_or_http()
            .enable_http1()
            .build();

        let client = Self {
            client: Client::builder(TokioExecutor::new()).build(conn),
        };

        Ok(client)
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
        let body = res
            .into_body()
            .collect()
            .await
            .map_err(|e| Error::GetBodyAutoConfigError(uri.clone(), e))?
            .to_bytes();

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
