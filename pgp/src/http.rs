use futures::{stream, StreamExt};
use hyper::{client::HttpConnector, http::uri::InvalidUri, Client, Uri};
use hyper_rustls::HttpsConnector;
use log::{debug, warn};
use pgp::{Deserializable, SignedPublicKey};
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
    ParsePublicKeyError(#[source] pgp::errors::Error, Uri),
    #[error("cannot find pgp public key for email {0}")]
    FindPublicKeyError(String),
}

async fn fetch(
    client: &Client<HttpsConnector<HttpConnector>>,
    email: &String,
    key_server: &String,
) -> Result<SignedPublicKey> {
    let uri: Uri = key_server
        .replace("<email>", email)
        .parse()
        .map_err(|err| Error::ParseUriError(err, key_server.clone()))?;

    let uri = match uri.scheme_str() {
        Some("hkp") | Some("hkps") => hkp::format_key_server_uri(uri, email).unwrap(),
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
        .map_err(|err| Error::ParsePublicKeyError(err, uri.clone()))?;

    Ok(pkey)
}

async fn get(
    client: &Client<HttpsConnector<HttpConnector>>,
    email: &String,
    key_servers: &[String],
) -> Result<SignedPublicKey> {
    for key_server in key_servers {
        match fetch(&client, &email, &key_server).await {
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
                    let msg = format!("cannot get pgp public keys as async stream");
                    warn!("{msg}: {err}");
                    debug!("{msg}: {err:?}");
                    None
                }
            }
        })
        .collect()
        .await
}
