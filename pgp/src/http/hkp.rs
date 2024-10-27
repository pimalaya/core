//! # HKP key discovery
//!
//! Module dedicated to HTTP Keyserver Protocol. Since HKP is just
//! HTTP, this module only contains a function that formats a given
//! URI to match [HKP specs].
//!
//! [HKP specs]: https://datatracker.ietf.org/doc/html/draft-shaw-openpgp-hkp-00

use http::ureq::http::Uri;

use crate::{Error, Result};

/// Formats the given URI to match the HKP specs.
///
/// It basically adds `/pks` plus few query params.
pub(crate) fn format_key_server_uri(uri: Uri, email: &str) -> Result<Uri> {
    let authority = uri.host().unwrap_or("localhost");
    let scheme = match uri.scheme_str() {
        Some("hkps") => "https",
        _ => "http",
    };

    let pks_path = format!("pks/lookup?op=get&search={email}");
    let path = if uri.path().is_empty() {
        String::from("/") + &pks_path
    } else {
        uri.path().to_owned() + &pks_path
    };

    let uri = Uri::builder()
        .scheme(scheme)
        .authority(authority)
        .path_and_query(path)
        .build()
        .map_err(|err| Error::BuildKeyServerUriError(err.into(), uri))?;

    Ok(uri)
}
