use std::env;

use pimalaya_oauth2::AuthorizationCodeGrant;

pub fn main() {
    let client_id = env::var("CLIENT_ID").expect("Missing the CLIENT_ID environment variable.");
    let client_secret =
        env::var("CLIENT_SECRET").expect("Missing the CLIENT_SECRET environment variable.");

    AuthorizationCodeGrant::new(
        client_id,
        client_secret,
        "https://accounts.google.com/o/oauth2/v2/auth",
        "https://www.googleapis.com/oauth2/v3/token",
    )
    .unwrap()
    .with_pkce(true)
    .with_scope("https://www.googleapis.com/auth/gmail.readonly")
    .execute()
    .unwrap();
}
