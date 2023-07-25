use hyper::{client::HttpConnector, http::uri::InvalidUri, Client, Uri};
use hyper_tls::HttpsConnector;
use log::{debug, warn};
use pgp::{Deserializable, SignedPublicKey};
use std::io::Cursor;
use thiserror::Error;

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

pub async fn get(
    email: impl AsRef<str>,
    keyservers: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<SignedPublicKey> {
    let email = email.as_ref();
    let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

    for keyserver in keyservers {
        let keyserver = keyserver.as_ref();
        match get_from_keyserver(&client, email, keyserver).await {
            Ok(pkey) => return Ok(pkey),
            Err(err) => {
                warn!("cannot get public key for {email} from {keyserver}: {err}");
                debug!("cannot get public key for {email} from {keyserver}: {err:?}");
                continue;
            }
        }
    }

    Ok(Err(Error::FindPublicKeyError(email.to_owned()))?)
}

async fn get_from_keyserver(
    client: &Client<HttpsConnector<HttpConnector>>,
    email: impl AsRef<str>,
    keyserver: impl AsRef<str>,
) -> Result<SignedPublicKey> {
    let email = email.as_ref();
    let keyserver = keyserver.as_ref();

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
