//! Module dedicated to HTTP Keyserver Protocol.
//!
//! Since HKP is just HTTP, this module only contains a function that
//! formats a given URI to match [HKP specs].
//!
//! [HKP specs]: https://datatracker.ietf.org/doc/html/draft-shaw-openpgp-hkp-00

use hyper::http::{
    uri::{Builder as UriBuilder, Uri},
    Result,
};

/// Formats the given URI to match the HKP specs.
///
/// It basically add `/pks` plus few query params.
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

    UriBuilder::new()
        .scheme(scheme)
        .authority(authority)
        .path_and_query(path)
        .build()
}
