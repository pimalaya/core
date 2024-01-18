//! # Account config discovery
//!
//! This module contains everything needed to discover account
//! configuration from a simple email address, based on the
//! Thunderbird [Autoconfiguration] standard.
//!
//! *NOTE: only IMAP and SMTP configurations can be discovered by this
//! module.*
//!
//! [Autoconfiguration]: https://udn.realityripple.com/docs/Mozilla/Thunderbird/Autoconfiguration#Mechanisms

use anyhow::bail;
use bytes::{Buf, Bytes};
use hyper::{body::to_bytes, client::HttpConnector, Client, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};

use crate::Result;

use super::config::AutoConfig;

pub struct Http {
    client: Client<HttpsConnector<HttpConnector>>,
}

impl Http {
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

    /// Fetches a given url and returns the XML response (if there is one)
    async fn get(&self, uri: Uri) -> Result<Bytes> {
        let res = self.client.get(uri).await?;

        let is_err = !res.status().is_success();
        let body = to_bytes(res.into_body()).await?;

        // If we got an error response we return an error
        if is_err {
            let err = String::from_utf8_lossy(&body);
            bail!("{err}")
        } else {
            Ok(body.into())
        }
    }

    pub async fn get_config(&self, uri: Uri) -> Result<AutoConfig> {
        let bytes = self.get(uri).await?;
        let config = serde_xml_rs::from_reader(bytes.reader())?;
        Ok(config)
    }
}
