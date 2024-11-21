//! Module dedicated to OAuth 2.0 configuration.
//!
//! This module contains everything related to OAuth 2.0
//! configuration.

use std::{fmt, io, net::TcpListener, vec};

use oauth::v2_0::{AuthorizationCodeGrant, Client, RefreshAccessToken};
use secret::Secret;
use tracing::debug;

#[doc(inline)]
pub use super::{Error, Result};

/// The OAuth 2.0 configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct OAuth2Config {
    /// Method for presenting an OAuth 2.0 bearer token to a service
    /// for authentication.
    pub method: OAuth2Method,

    /// Client identifier issued to the client during the registration process described by
    /// [Section 2.2](https://datatracker.ietf.org/doc/html/rfc6749#section-2.2).
    pub client_id: String,

    /// Client password issued to the client during the registration process described by
    /// [Section 2.2](https://datatracker.ietf.org/doc/html/rfc6749#section-2.2).
    pub client_secret: Option<Secret>,

    /// URL of the authorization server's authorization endpoint.
    pub auth_url: String,

    /// URL of the authorization server's token endpoint.
    pub token_url: String,

    /// Access token returned by the token endpoint and used to access
    /// protected resources.
    #[cfg_attr(
        feature = "derive",
        serde(default, skip_serializing_if = "Secret::is_empty")
    )]
    pub access_token: Secret,

    /// Refresh token used to obtain a new access token (if supported
    /// by the authorization server).
    #[cfg_attr(
        feature = "derive",
        serde(default, skip_serializing_if = "Secret::is_empty")
    )]
    pub refresh_token: Secret,

    /// Enable the [PKCE](https://datatracker.ietf.org/doc/html/rfc7636) protection.
    /// The value must have a minimum length of 43 characters and a maximum length of 128 characters.
    /// Each character must be ASCII alphanumeric or one of the characters “-” / “.” / “_” / “~”.
    pub pkce: bool,

    pub redirect_scheme: Option<String>,
    pub redirect_host: Option<String>,
    pub redirect_port: Option<u16>,

    /// Access token scope(s), as defined by the authorization server.
    #[cfg_attr(feature = "derive", serde(flatten))]
    pub scopes: OAuth2Scopes,
}

impl OAuth2Config {
    pub const LOCALHOST: &'static str = "localhost";

    /// Return the first available port on [`LOCALHOST`].
    pub fn get_first_available_port() -> Result<u16> {
        (49_152..65_535)
            .find(|port| TcpListener::bind((OAuth2Config::LOCALHOST, *port)).is_ok())
            .ok_or(Error::GetAvailablePortError)
    }

    /// Resets the three secrets of the OAuth 2.0 configuration.
    pub async fn reset(&self) -> Result<()> {
        if let Some(secret) = self.client_secret.as_ref() {
            secret
                .delete_if_keyring()
                .await
                .map_err(Error::DeleteClientSecretOauthError)?;
        }

        self.access_token
            .delete_if_keyring()
            .await
            .map_err(Error::DeleteAccessTokenOauthError)?;
        self.refresh_token
            .delete_if_keyring()
            .await
            .map_err(Error::DeleteRefreshTokenOauthError)?;

        Ok(())
    }

    /// If the access token is not defined, runs the authorization
    /// code grant OAuth 2.0 flow in order to save the acces token and
    /// the refresh token if present.
    pub async fn configure(
        &self,
        get_client_secret: impl Fn() -> io::Result<String>,
    ) -> Result<()> {
        if self.access_token.get().await.is_ok() {
            return Ok(());
        }

        let redirect_scheme = match self.redirect_scheme.as_ref() {
            Some(scheme) => scheme.clone(),
            None => "http".into(),
        };

        let redirect_host = match self.redirect_host.as_ref() {
            Some(host) => host.clone(),
            None => OAuth2Config::LOCALHOST.to_owned(),
        };

        let redirect_port = match self.redirect_port {
            Some(port) => port,
            None => OAuth2Config::get_first_available_port()?,
        };

        let client_secret = match self.client_secret.as_ref() {
            None => None,
            Some(secret) => Some(match secret.find().await {
                Ok(None) => {
                    debug!("cannot find oauth2 client secret from keyring, setting it");
                    secret
                        .set_if_keyring(
                            get_client_secret()
                                .map_err(Error::GetClientSecretFromUserOauthError)?,
                        )
                        .await
                        .map_err(Error::SetClientSecretIntoKeyringOauthError)
                }
                Ok(Some(client_secret)) => Ok(client_secret),
                Err(err) => Err(Error::GetClientSecretFromKeyringOauthError(err)),
            }?),
        };

        let client = Client::new(
            self.client_id.clone(),
            client_secret,
            self.auth_url.clone(),
            self.token_url.clone(),
            redirect_scheme,
            redirect_host,
            redirect_port,
        )
        .map_err(Error::BuildOauthClientError)?;

        let mut auth_code_grant = AuthorizationCodeGrant::new();

        if self.pkce {
            auth_code_grant = auth_code_grant.with_pkce();
        }

        for scope in self.scopes.clone() {
            auth_code_grant = auth_code_grant.with_scope(scope);
        }

        let (redirect_url, csrf_token) = auth_code_grant.get_redirect_url(&client);

        println!("To complete your OAuth 2.0 setup, click on the following link:");
        println!();
        println!("{}", redirect_url);

        let (access_token, refresh_token) = auth_code_grant
            .wait_for_redirection(&client, csrf_token)
            .await
            .map_err(Error::WaitForOauthRedirectionError)?;

        self.access_token
            .set_if_keyring(access_token)
            .await
            .map_err(Error::SetAccessTokenOauthError)?;

        if let Some(refresh_token) = &refresh_token {
            self.refresh_token
                .set_if_keyring(refresh_token)
                .await
                .map_err(Error::SetRefreshTokenOauthError)?;
        }

        Ok(())
    }

    /// Runs the refresh access token OAuth 2.0 flow by exchanging a
    /// refresh token with a new pair of access/refresh token.
    pub async fn refresh_access_token(&self) -> Result<String> {
        let redirect_scheme = match self.redirect_scheme.as_ref() {
            Some(scheme) => scheme.clone(),
            None => "http".into(),
        };

        let redirect_host = match self.redirect_host.as_ref() {
            Some(host) => host.clone(),
            None => OAuth2Config::LOCALHOST.to_owned(),
        };

        let redirect_port = match self.redirect_port {
            Some(port) => port,
            None => OAuth2Config::get_first_available_port()?,
        };

        let client_secret = match self.client_secret.as_ref() {
            None => None,
            Some(secret) => {
                let secret = secret
                    .get()
                    .await
                    .map_err(Error::GetClientSecretFromKeyringOauthError)?;
                Some(secret)
            }
        };

        let client = Client::new(
            self.client_id.clone(),
            client_secret,
            self.auth_url.clone(),
            self.token_url.clone(),
            redirect_scheme,
            redirect_host,
            redirect_port,
        )
        .map_err(Error::BuildOauthClientError)?;

        let refresh_token = self
            .refresh_token
            .get()
            .await
            .map_err(Error::GetRefreshTokenOauthError)?;

        let (access_token, refresh_token) = RefreshAccessToken::new()
            .refresh_access_token(&client, refresh_token)
            .await
            .map_err(Error::RefreshAccessTokenOauthError)?;

        self.access_token
            .set_if_keyring(&access_token)
            .await
            .map_err(Error::SetAccessTokenOauthError)?;

        if let Some(refresh_token) = &refresh_token {
            self.refresh_token
                .set_if_keyring(refresh_token)
                .await
                .map_err(Error::SetRefreshTokenOauthError)?;
        }

        Ok(access_token)
    }

    /// Returns the access token if existing, otherwise returns an
    /// error.
    pub async fn access_token(&self) -> Result<String> {
        self.access_token
            .get()
            .await
            .map_err(Error::GetAccessTokenOauthError)
    }
}

/// Method for presenting an OAuth 2.0 bearer token to a service for
/// authentication.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum OAuth2Method {
    #[default]
    #[cfg_attr(feature = "derive", serde(alias = "XOAUTH2"))]
    XOAuth2,
    #[cfg_attr(feature = "derive", serde(alias = "OAUTHBEARER"))]
    OAuthBearer,
}

impl fmt::Display for OAuth2Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::XOAuth2 => write!(f, "XOAUTH2"),
            Self::OAuthBearer => write!(f, "OAUTHBEARER"),
        }
    }
}

/// Access token scope(s), as defined by the authorization server.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum OAuth2Scopes {
    Scope(String),
    Scopes(Vec<String>),
}

impl Default for OAuth2Scopes {
    fn default() -> Self {
        Self::Scopes(Vec::new())
    }
}

impl IntoIterator for OAuth2Scopes {
    type IntoIter = vec::IntoIter<Self::Item>;
    type Item = String;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Scope(scope) => vec![scope].into_iter(),
            Self::Scopes(scopes) => scopes.into_iter(),
        }
    }
}
