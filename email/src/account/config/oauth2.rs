//! Module dedicated to OAuth 2.0 configuration.
//!
//! This module contains everything related to OAuth 2.0
//! configuration.

use log::warn;
use oauth::v2_0::{AuthorizationCodeGrant, Client, RefreshAccessToken};
use secret::Secret;
use std::{io, net::TcpListener, vec};
use thiserror::Error;

use crate::Result;

/// Errors related to OAuth 2.0 configuration.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot create oauth2 client")]
    InitClientError(#[source] oauth::v2_0::Error),
    #[error("cannot create oauth2 client")]
    BuildClientError(#[source] oauth::v2_0::Error),
    #[error("cannot wait for oauth2 redirection error")]
    WaitForRedirectionError(#[source] oauth::v2_0::Error),

    #[error("cannot get oauth2 access token from global keyring")]
    GetAccessTokenError(#[source] secret::Error),
    #[error("cannot set oauth2 access token")]
    SetAccessTokenError(#[source] secret::Error),
    #[error("cannot refresh oauth2 access token")]
    RefreshAccessTokenError(#[source] oauth::v2_0::Error),
    #[error("cannot delete oauth2 access token from global keyring")]
    DeleteAccessTokenError(#[source] secret::Error),

    #[error("cannot get oauth2 refresh token")]
    GetRefreshTokenError(#[source] secret::Error),
    #[error("cannot set oauth2 refresh token")]
    SetRefreshTokenError(#[source] secret::Error),
    #[error("cannot delete oauth2 refresh token from global keyring")]
    DeleteRefreshTokenError(#[source] secret::Error),

    #[error("cannot get oauth2 client secret from user")]
    GetClientSecretFromUserError(#[source] io::Error),
    #[error("cannot get oauth2 client secret from global keyring")]
    GetClientSecretFromKeyringError(#[source] secret::Error),
    #[error("cannot save oauth2 client secret into global keyring")]
    SetClientSecretIntoKeyringError(#[source] secret::Error),
    #[error("cannot delete oauth2 client secret from global keyring")]
    DeleteClientSecretError(#[source] secret::Error),

    #[error("cannot get available port")]
    GetAvailablePortError,
}

/// The OAuth 2.0 configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuth2Config {
    /// Method for presenting an OAuth 2.0 bearer token to a service
    /// for authentication.
    pub method: OAuth2Method,

    /// Client identifier issued to the client during the registration process described by
    /// [Section 2.2](https://datatracker.ietf.org/doc/html/rfc6749#section-2.2).
    pub client_id: String,

    /// Client password issued to the client during the registration process described by
    /// [Section 2.2](https://datatracker.ietf.org/doc/html/rfc6749#section-2.2).
    pub client_secret: Secret,

    /// URL of the authorization server's authorization endpoint.
    pub auth_url: String,

    /// URL of the authorization server's token endpoint.
    pub token_url: String,

    /// Access token returned by the token endpoint and used to access
    /// protected resources.
    pub access_token: Secret,

    /// Refresh token used to obtain a new access token (if supported
    /// by the authorization server).
    pub refresh_token: Secret,

    /// Enable the [PKCE](https://datatracker.ietf.org/doc/html/rfc7636) protection.
    /// The value must have a minimum length of 43 characters and a maximum length of 128 characters.
    /// Each character must be ASCII alphanumeric or one of the characters “-” / “.” / “_” / “~”.
    pub pkce: bool,

    /// Access token scope(s), as defined by the authorization server.
    pub scopes: OAuth2Scopes,

    /// Host name of the client's redirection endpoint.
    pub redirect_host: String,

    /// Host port of the client's redirection endpoint.
    pub redirect_port: u16,
}

impl Default for OAuth2Config {
    fn default() -> Self {
        Self {
            method: Default::default(),
            client_id: Default::default(),
            client_secret: Default::default(),
            auth_url: Default::default(),
            token_url: Default::default(),
            access_token: Default::default(),
            refresh_token: Default::default(),
            pkce: Default::default(),
            scopes: Default::default(),
            redirect_host: Self::default_redirect_host(),
            redirect_port: Self::default_redirect_port(),
        }
    }
}

impl OAuth2Config {
    pub fn new() -> Result<Self> {
        Ok(Self {
            redirect_port: (49_152..65_535)
                .find(|port| TcpListener::bind((Self::default_redirect_host(), *port)).is_ok())
                .ok_or(Error::GetAvailablePortError)?,
            ..Default::default()
        })
    }

    /// Returns the default redirect host name. Combines well with
    /// serde's `default` and `skip_serializing_if` macros.
    pub fn default_redirect_host() -> String {
        String::from("localhost")
    }

    /// Returns the default redirect host port. Combines well with
    /// serde's `default` and `skip_serializing_if` macros.
    pub fn default_redirect_port() -> u16 {
        9999
    }

    /// Resets the three secrets of the OAuth 2.0 configuration.
    pub fn reset(&self) -> Result<()> {
        self.client_secret
            .delete_keyring_entry_secret()
            .map_err(Error::DeleteClientSecretError)?;
        self.access_token
            .delete_keyring_entry_secret()
            .map_err(Error::DeleteAccessTokenError)?;
        self.refresh_token
            .delete_keyring_entry_secret()
            .map_err(Error::DeleteRefreshTokenError)?;
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

        let set_client_secret = || {
            self.client_secret
                .set_keyring_entry_secret(
                    get_client_secret().map_err(Error::GetClientSecretFromUserError)?,
                )
                .map_err(Error::SetClientSecretIntoKeyringError)
        };

        let client_secret = match self.client_secret.find().await {
            Ok(None) => {
                warn!("cannot find oauth2 client secret from keyring, setting it");
                set_client_secret()
            }
            Ok(Some(client_secret)) => Ok(client_secret),
            Err(err) => Err(Error::GetClientSecretFromKeyringError(err)),
        }?;

        let client = Client::new(
            self.client_id.clone(),
            client_secret,
            self.auth_url.clone(),
            self.token_url.clone(),
        )
        .map_err(Error::InitClientError)?
        .with_redirect_host(self.redirect_host.clone())
        .with_redirect_port(self.redirect_port)
        .build()
        .map_err(Error::BuildClientError)?;

        let mut auth_code_grant = AuthorizationCodeGrant::new()
            .with_redirect_host(self.redirect_host.clone())
            .with_redirect_port(self.redirect_port);

        if self.pkce {
            auth_code_grant = auth_code_grant.with_pkce();
        }

        for scope in self.scopes.clone() {
            auth_code_grant = auth_code_grant.with_scope(scope);
        }

        let (redirect_url, csrf_token) = auth_code_grant.get_redirect_url(&client);

        println!("To complete your OAuth 2.0 setup, click on the following link:");
        println!("");
        println!("{}", redirect_url.to_string());

        let (access_token, refresh_token) = auth_code_grant
            .wait_for_redirection(&client, csrf_token)
            .await
            .map_err(Error::WaitForRedirectionError)?;

        self.access_token
            .set_keyring_entry_secret(access_token)
            .map_err(Error::SetAccessTokenError)?;

        if let Some(refresh_token) = &refresh_token {
            self.refresh_token
                .set_keyring_entry_secret(refresh_token)
                .map_err(Error::SetRefreshTokenError)?;
        }

        Ok(())
    }

    /// Runs the refresh access token OAuth 2.0 flow by exchanging a
    /// refresh token with a new pair of access/refresh token.
    pub async fn refresh_access_token(&self) -> Result<String> {
        let client_secret = self
            .client_secret
            .get()
            .await
            .map_err(Error::GetClientSecretFromKeyringError)?;

        let client = Client::new(
            self.client_id.clone(),
            client_secret,
            self.auth_url.clone(),
            self.token_url.clone(),
        )
        .map_err(Error::InitClientError)?
        .with_redirect_host(self.redirect_host.clone())
        .with_redirect_port(self.redirect_port)
        .build()
        .map_err(Error::BuildClientError)?;

        let refresh_token = self
            .refresh_token
            .get()
            .await
            .map_err(Error::GetRefreshTokenError)?;

        let (access_token, refresh_token) = RefreshAccessToken::new()
            .refresh_access_token(&client, refresh_token)
            .await
            .map_err(Error::RefreshAccessTokenError)?;

        self.access_token
            .set_keyring_entry_secret(&access_token)
            .map_err(Error::SetAccessTokenError)?;

        if let Some(refresh_token) = &refresh_token {
            self.refresh_token
                .set_keyring_entry_secret(refresh_token)
                .map_err(Error::SetRefreshTokenError)?;
        }

        Ok(access_token)
    }

    /// Returns the access token if existing, otherwise returns an
    /// error.
    pub async fn access_token(&self) -> Result<String> {
        let access_token = self
            .access_token
            .get()
            .await
            .map_err(Error::GetAccessTokenError)?;
        Ok(access_token)
    }
}

/// Method for presenting an OAuth 2.0 bearer token to a service for
/// authentication.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum OAuth2Method {
    #[default]
    XOAuth2,
    OAuthBearer,
}

/// Access token scope(s), as defined by the authorization server.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    type Item = String;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Scope(scope) => vec![scope].into_iter(),
            Self::Scopes(scopes) => scopes.into_iter(),
        }
    }
}
