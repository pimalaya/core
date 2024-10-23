//! Refresh Access Token flow helper, as defined in the
//! [RFC6749](https://datatracker.ietf.org/doc/html/rfc6749#section-6)

use oauth2::{RefreshToken, TokenResponse};

use super::{Client, Error, Result};

/// OAuth 2.0 Refresh Access Token flow builder. The builder is empty
/// for now but scopes will be added in the future. This flow exchange
/// a refresh token for a new pair of access token and maybe a refresh
/// token.
#[derive(Debug, Default)]
pub struct RefreshAccessToken;

impl RefreshAccessToken {
    pub fn new() -> Self {
        Self
    }

    pub async fn refresh_access_token(
        &self,
        client: &Client,
        refresh_token: impl ToString,
    ) -> Result<(String, Option<String>)> {
        let res = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .request_async(&Client::send_oauth2_request)
            .await
            .map_err(Box::new)
            .map_err(Error::RefreshAccessTokenError)?;

        let access_token = res.access_token().secret().to_owned();
        let refresh_token = res.refresh_token().map(|t| t.secret().clone());

        Ok((access_token, refresh_token))
    }
}
