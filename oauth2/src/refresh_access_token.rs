//! Refresh Access Token flow helper, as defined in the
//! [RFC6749](https://datatracker.ietf.org/doc/html/rfc6749#section-6)

use oauth2::{
    basic::{BasicClient, BasicErrorResponseType},
    reqwest::http_client,
    RefreshToken, RequestTokenError, StandardErrorResponse, TokenResponse,
};
use std::result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot refresh access token using the refresh token")]
    RefreshAccessTokenError(
        RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ),
}

pub type Result<T> = result::Result<T, Error>;

/// OAuth 2.0 Refresh Access Token flow builder. The builder is empty
/// for now but scopes will be added in the future. This flow exchange
/// a refresh token for a new pair of access token and refresh token.
#[derive(Debug)]
pub struct RefreshAccessToken;

impl RefreshAccessToken {
    pub fn new() -> Self {
        Self
    }

    pub fn refresh_access_token(
        &self,
        client: &BasicClient,
        refresh_token: impl ToString,
    ) -> Result<(String, Option<String>)> {
        let res = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
            .request(http_client)
            .map_err(Error::RefreshAccessTokenError)?;

        let access_token = res.access_token().secret().to_owned();
        let refresh_token = res.refresh_token().map(|t| t.secret().clone());

        Ok((access_token, refresh_token))
    }
}
