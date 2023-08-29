use oauth::v2_0::{AuthorizationCodeGrant, Client, RefreshAccessToken};
use std::env;

#[tokio::main]
pub async fn main() {
    let port = env::var("PORT")
        .unwrap_or(String::from("9999"))
        .parse::<u16>()
        .expect("Invalid port");
    let client_id = env::var("CLIENT_ID").expect("Missing the CLIENT_ID environment variable");
    let client_secret =
        env::var("CLIENT_SECRET").expect("Missing the CLIENT_SECRET environment variable");

    let client = Client::new(
        client_id,
        client_secret,
        "https://accounts.google.com/o/oauth2/v2/auth",
        "https://www.googleapis.com/oauth2/v3/token",
    )
    .unwrap()
    .with_redirect_port(port)
    .build()
    .unwrap();

    let auth_code_grant = AuthorizationCodeGrant::new()
        .with_redirect_port(port)
        .with_pkce()
        .with_scope("https://mail.google.com/");

    let (redirect_url, csrf_token) = auth_code_grant.get_redirect_url(&client);

    println!("Go to: {}", redirect_url.to_string());

    let (access_token, refresh_token) = auth_code_grant
        .wait_for_redirection(&client, csrf_token)
        .await
        .unwrap();

    println!("access token: {:?}", access_token);
    println!("refresh token: {:?}", refresh_token);

    if let Some(refresh_token) = refresh_token {
        let (access_token, refresh_token) = RefreshAccessToken::new()
            .refresh_access_token(&client, refresh_token)
            .await
            .unwrap();

        println!("new access token: {:?}", access_token);
        println!("new refresh token: {:?}", refresh_token);
    }
}