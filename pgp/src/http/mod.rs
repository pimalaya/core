//! Module dedicated to HTTP.
//!
//! The main purpose of this module is to get public keys belonging to
//! given emails by contacting key servers.

pub mod hkp;
pub mod wkd;

use std::{io::Cursor, sync::Arc};

use futures::{stream, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use native::{Deserializable, SignedPublicKey};
use tokio::task;
use tracing::{debug, warn};

use crate::{Error, Result};

pub type HttpClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

/// Builds a new HTTP client.
pub(crate) fn new_http_client() -> Result<Arc<HttpClient>> {
    let conn = HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(Error::CreateHttpConnectorError)?
        .https_or_http()
        .enable_http1()
        .build();

    let client = Client::builder(TokioExecutor::new()).build(conn);

    Ok(Arc::new(client))
}

/// Calls the given key server in order to get the public key
/// belonging to the given email address.
async fn fetch(client: &HttpClient, email: &str, key_server: &str) -> Result<SignedPublicKey> {
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

    let status = res.status();
    let body = res
        .into_body()
        .collect()
        .await
        .map_err(|err| Error::ParseBodyWithUriError(err, uri.clone()))?
        .to_bytes();

    if !status.is_success() {
        let err = String::from_utf8_lossy(&body).to_string();
        return Err(Error::GetPublicKeyError(uri.clone(), status, err));
    }

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
    client: &HttpClient,
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

    Err(Error::FindPublicKeyError(email.to_owned()))
}

/// Gets public key associated to the given email.
pub async fn get_one(email: String, key_servers: Vec<String>) -> Result<SignedPublicKey> {
    let client = new_http_client()?;
    self::get(&client, &email, &key_servers).await
}

/// Gets public keys associated to the given emails.
pub async fn get_all(
    emails: Vec<String>,
    key_servers: Vec<String>,
) -> Result<Vec<(String, Result<SignedPublicKey>)>> {
    let key_servers = Arc::new(key_servers);
    let client = new_http_client()?;

    let pkeys = stream::iter(emails)
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
                    debug!("{msg}: {err:?}");
                    None
                }
            }
        })
        .collect()
        .await;

    Ok(pkeys)
}
