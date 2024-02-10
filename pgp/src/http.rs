//! Module dedicated to HTTP.
//!
//! The main purpose of this module is to get public keys belonging to
//! given emails by contacting key servers.

use futures::{stream, StreamExt};
use hyper::{client::HttpConnector, http::uri::InvalidUri, Client, Uri};
use hyper_rustls::HttpsConnector;
use log::{debug, warn};
use pgp_native::{Deserializable, SignedPublicKey};
use std::{io::Cursor, sync::Arc};
use thiserror::Error;
use tokio::task;

use crate::{client, hkp, Result};

/// Errors related to HTTP.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse uri {1}")]
    ParseUriError(#[source] InvalidUri, String),
    #[error("cannot parse body from {1}")]
    ParseBodyError(#[source] hyper::Error, Uri),
    #[error("cannot parse response from {1}")]
    FetchResponseError(#[source] hyper::Error, Uri),
    #[error("cannot parse pgp public key from {1}")]
    ParsePublicKeyError(#[source] pgp_native::errors::Error, Uri),
    #[error("cannot find pgp public key for email {0}")]
    FindPublicKeyError(String),
}

/// Calls the given key server in order to get the public key
/// belonging to the given email address.
async fn fetch(
    client: &Client<HttpsConnector<HttpConnector>>,
    email: &str,
    key_server: &str,
) -> Result<SignedPublicKey> {
    let uri: Uri = key_server
        .replace("<email>", email)
        .parse()
        .map_err(|err| Error::ParseUriError(err, key_server.to_owned()))?;

    let uri = match uri.scheme_str() {
        Some("hkp") | Some("hkps") => hkp::format_key_server_uri(uri, email).unwrap(),
        // TODO: manage file scheme
        _ => uri,
    };

    let res = client
        .get(uri.clone())
        .await
        .map_err(|err| Error::FetchResponseError(err, uri.clone()))?;

    let body = hyper::body::to_bytes(res.into_body())
        .await
        .map_err(|err| Error::ParseBodyError(err, uri.clone()))?;

    let cursor = Cursor::new(&*body);
    let (pkey, _) = SignedPublicKey::from_armor_single(cursor)
        .map_err(|err| Error::ParsePublicKeyError(err, uri))?;

    Ok(pkey)
}

/// Calls the given key servers synchronously and stops when a public
/// key belonging to the given email address is found.
///
/// A better algorithm would be to contact asynchronously all key
/// servers and to abort pending futures when a public key is found.
async fn get(
    client: &Client<HttpsConnector<HttpConnector>>,
    email: &String,
    key_servers: &[String],
) -> Result<SignedPublicKey> {
    for key_server in key_servers {
        match fetch(client, email, key_server).await {
            Ok(pkey) => {
                debug!("found pgp public key for {email} at {key_server}");
                return Ok(pkey);
            }
            Err(err) => {
                let msg = format!("cannot get pgp public key for {email} at {key_server}");
                warn!("{msg}: {err}");
                debug!("{msg}: {err:?}");
                continue;
            }
        }
    }

    Ok(Err(Error::FindPublicKeyError(email.to_owned()))?)
}

/// Gets public key associated to the given email.
pub async fn get_one(email: String, key_servers: Vec<String>) -> Result<SignedPublicKey> {
    let client = client::build();
    self::get(&client, &email, &key_servers).await
}

/// Gets public keys associated to the given emails.
pub async fn get_all(
    emails: Vec<String>,
    key_servers: Vec<String>,
) -> Vec<(String, Result<SignedPublicKey>)> {
    let key_servers = Arc::new(key_servers);
    let client = client::build();

    stream::iter(emails)
        .map(|email| {
            let key_servers = key_servers.clone();
            let client = client.clone();
            task::spawn(async move {
                (
                    email.clone(),
                    self::get(&client, &email, &key_servers).await,
                )
            })
        })
        .buffer_unordered(8)
        .filter_map(|res| async {
            match res {
                Ok(res) => Some(res),
                Err(err) => {
                    let msg = "cannot get pgp public keys as async stream".to_owned();
                    warn!("{msg}: {err}");
                    debug!("{msg}: {err:?}");
                    None
                }
            }
        })
        .collect()
        .await
}
