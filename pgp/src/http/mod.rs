//! # HTTP key discovery
//!
//! Module dedicated to HTTP public key discovery. The main purpose of
//! this module is to get public keys belonging to given emails by
//! contacting key servers.

pub mod hkp;
pub mod wkd;

use std::{
    io::{Cursor, Read},
    sync::Arc,
};

use futures::{stream::FuturesUnordered, StreamExt};
use http::ureq::http::Uri;
use tracing::{debug, warn};

use crate::{
    native::{Deserializable, SignedPublicKey},
    utils::spawn,
    Error, Result,
};

/// Calls the given key server in order to get the public key
/// belonging to the given email address.
async fn fetch(client: &http::Client, email: &str, key_server: &str) -> Result<SignedPublicKey> {
    let uri: Uri = key_server
        .replace("<email>", email)
        .parse()
        .map_err(http::Error::from)?;

    let uri = match uri.scheme_str() {
        Some("hkp") | Some("hkps") => hkp::format_key_server_uri(uri, email).unwrap(),
        // TODO: manage file scheme
        _ => uri,
    };

    let uri_clone = uri.clone();
    let res = client
        .send(move |agent| agent.get(uri_clone).call())
        .await?;

    let status = res.status();
    let mut body = res.into_body();
    let mut body = body.as_reader();

    if !status.is_success() {
        let mut err = String::new();
        body.read_to_string(&mut err)
            .map_err(|err| Error::ReadHttpError(err, uri.clone(), status))?;
        return Err(Error::GetPublicKeyError(err, uri, status));
    }

    let mut bytes = Vec::new();
    body.read_to_end(&mut bytes)
        .map_err(|err| Error::ReadPublicKeyError(err, uri.clone()))?;
    let cursor = Cursor::new(bytes);
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
    client: &http::Client,
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
    let client = http::Client::new();
    self::get(&client, &email, &key_servers).await
}

/// Gets public keys associated to the given emails.
pub async fn get_all(
    emails: Vec<String>,
    key_servers: Vec<String>,
) -> Vec<(String, Result<SignedPublicKey>)> {
    let key_servers = Arc::new(key_servers);
    let client = http::Client::new();

    FuturesUnordered::from_iter(emails.into_iter().map(|email| {
        let key_servers = key_servers.clone();
        let client = client.clone();
        spawn(async move {
            (
                email.clone(),
                self::get(&client, &email, &key_servers).await,
            )
        })
    }))
    .filter_map(|res| async {
        match res {
            Ok(res) => {
                return Some(res);
            }
            Err(err) => {
                debug!(?err, "skipping failed task");
                None
            }
        }
    })
    .collect()
    .await
}
