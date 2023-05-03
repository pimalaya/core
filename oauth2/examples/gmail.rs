use std::env;

use pimalaya_oauth2::AuthorizationCodeGrant;

pub fn main() {
    let client_id = env::var("CLIENT_ID").expect("Missing the CLIENT_ID environment variable.");
    let client_secret =
        env::var("CLIENT_SECRET").expect("Missing the CLIENT_SECRET environment variable.");

    let builder = AuthorizationCodeGrant::new(
        client_id,
        client_secret,
        "https://accounts.google.com/o/oauth2/v2/auth",
        "https://www.googleapis.com/oauth2/v3/token",
    )
    .unwrap()
    .with_pkce()
    .with_scope("https://www.googleapis.com/auth/gmail.readonly");

    let client = builder.get_client().unwrap();
    let (redirect_url, csrf_token) = builder.get_redirect_url(&client);

    println!("Go to: {}", redirect_url.to_string());

    let (access_token, refresh_token) = builder.start_redirect_server(client, csrf_token).unwrap();

    println!("access token: {:?}", access_token);
    println!("refresh token: {:?}", refresh_token);
}
