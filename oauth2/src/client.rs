//! Client builder, used by other flows to send requests and build
//! URLs.

use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use thiserror::Error;

use crate::Result;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot build auth url")]
    BuildAuthUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot build token url")]
    BuildTokenUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot build redirect url")]
    BuildRedirectUrlError(#[source] oauth2::url::ParseError),
}

/// Client builder, used by other flows to send requests and build
/// URLs.
#[derive(Debug)]
pub struct Client {
    /// Client identifier issued to the client during the registration process described by
    /// [Section 2.2](https://tools.ietf.org/html/rfc6749#section-2.2).
    pub client_id: ClientId,

    /// Client password issued to the client during the registration process described by
    /// [Section 2.2](https://tools.ietf.org/html/rfc6749#section-2.2).
    pub client_secret: ClientSecret,

    /// URL of the authorization server's authorization endpoint.
    pub auth_url: AuthUrl,

    /// URL of the authorization server's token endpoint.
    pub token_url: TokenUrl,

    /// Hostname of the client's redirection endpoint.
    pub redirect_host: String,

    /// Port of the client's redirection endpoint.
    pub redirect_port: u16,
}

impl Client {
    pub fn new(
        client_id: impl ToString,
        client_secret: impl ToString,
        auth_url: impl ToString,
        token_url: impl ToString,
    ) -> Result<Self> {
        Ok(Self {
            client_id: ClientId::new(client_id.to_string()),
            client_secret: ClientSecret::new(client_secret.to_string()),
            auth_url: AuthUrl::new(auth_url.to_string()).map_err(Error::BuildAuthUrlError)?,
            token_url: TokenUrl::new(token_url.to_string()).map_err(Error::BuildTokenUrlError)?,
            redirect_host: String::from("localhost"),
            redirect_port: 9999,
        })
    }

    pub fn with_redirect_host<T>(mut self, host: T) -> Self
    where
        T: ToString,
    {
        self.redirect_host = host.to_string();
        self
    }

    pub fn with_redirect_port<T>(mut self, port: T) -> Self
    where
        T: Into<u16>,
    {
        self.redirect_port = port.into();
        self
    }

    /// Build the final client.
    pub fn build(&self) -> Result<BasicClient> {
        let host = &self.redirect_host;
        let port = self.redirect_port;
        let redirect_uri = RedirectUrl::new(format!("http://{host}:{port}"))
            .map_err(Error::BuildRedirectUrlError)?;

        let client = BasicClient::new(
            self.client_id.clone(),
            Some(self.client_secret.clone()),
            self.auth_url.clone(),
            Some(self.token_url.clone()),
        )
        .set_redirect_uri(redirect_uri);

        Ok(client)
    }
}
