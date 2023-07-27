use futures::{stream, StreamExt};
use hyper::{client::HttpConnector, http::uri::InvalidUri, Client, Uri};
use hyper_tls::HttpsConnector;
use log::{debug, warn};
use pgp::{Deserializable, SignedPublicKey};
use std::{io::Cursor, sync::Arc};
use thiserror::Error;
use tokio::task;

use crate::Result;

/// Errors related to HKPS.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse uri {1}")]
    ParseUriError(#[source] InvalidUri, String),
    #[error("cannot parse body from {1}")]
    ParseBodyError(#[source] hyper::Error, Uri),
    #[error("cannot parse response from {1}")]
    GetResponseError(#[source] hyper::Error, Uri),
    #[error("cannot parse public key from {1}")]
    ParsePublicKeyError(#[source] pgp::errors::Error, Uri),
    #[error("cannot find public key for email {0}")]
    FindPublicKeyError(String),
}

async fn get_from_keyserver(
    client: &Client<HttpsConnector<HttpConnector>>,
    email: &String,
    keyserver: &String,
) -> Result<SignedPublicKey> {
    let uri = format!("https://{keyserver}/pks/lookup?op=get&search={email}");
    let uri: Uri = uri
        .parse()
        .map_err(|err| Error::ParseUriError(err, uri.clone()))?;

    let res = client
        .get(uri.clone())
        .await
        .map_err(|err| Error::GetResponseError(err, uri.clone()))?;

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
        match get_from_keyserver(&client, &email, &key_server).await {
            Ok(pkey) => return Ok(pkey),
            Err(err) => {
                warn!("cannot get public key for {email} from {key_server}: {err}");
                debug!("cannot get public key for {email} from {key_server}: {err:?}");
                continue;
            }
        }
    }

    Ok(Err(Error::FindPublicKeyError(email.to_owned()))?)
}

/// Gets public keys associated to the given emails.
pub async fn get_one(email: String, key_servers: Vec<String>) -> Result<SignedPublicKey> {
    let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());
    self::get(&client, &email, &key_servers).await
}

/// Gets public keys associated to the given emails.
pub async fn get_all(
    emails: Vec<String>,
    key_servers: Vec<String>,
) -> Vec<(String, Result<SignedPublicKey>)> {
    let key_servers = Arc::new(key_servers);
    let client = Arc::new(Client::builder().build::<_, hyper::Body>(HttpsConnector::new()));

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
                    warn!("cannot join async task: {err}");
                    debug!("cannot join async task: {err:?}");
                    None
                }
            }
        })
        .collect()
        .await
}
