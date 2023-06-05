//! Authorization Grant Code flow helper, as defined in the
//! [RFC6749](https://datatracker.ietf.org/doc/html/rfc6749#section-1.3.1)

use oauth2::{
    basic::{BasicClient, BasicErrorResponseType},
    url::{self, Url},
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RequestTokenError, Scope, StandardErrorResponse, TokenResponse,
    TokenUrl,
};
use std::{
    io::{self, prelude::*, BufReader},
    net::TcpListener,
    result,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot build auth url")]
    BuildAuthUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot build token url")]
    BuildTokenUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot build revocation url")]
    BuildRevocationUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot build redirect url")]
    BuildRedirectUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot bind redirect server")]
    BindRedirectServerError(String, u16, #[source] io::Error),
    #[error("cannot accept redirect server connections")]
    AcceptRedirectServerError(#[source] io::Error),
    #[error("invalid state {0}: expected {1}")]
    InvalidStateError(String, String),
    #[error("missing redirect url from {0}")]
    MissingRedirectUrlError(String),
    #[error("cannot parse redirect url {1}")]
    ParseRedirectUrlError(#[source] url::ParseError, String),
    #[error("cannot find code from redirect url {0}")]
    FindCodeInRedirectUrlError(Url),
    #[error("cannot find state from redirect url {0}")]
    FindStateInRedirectUrlError(Url),
    #[error("cannot exchange code for an access token and a refresh token")]
    ExchangeCodeError(
        RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ),

    #[error(transparent)]
    IoError(#[from] io::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// OAuth 2.0 Authorization Code Grant flow builder.
///
/// The first step (once the builder is configured) is to get a client
/// by calling [`AuthorizationCodeGrant::get_client`].
///
/// The second step is to get the redirect URL by calling
/// [`AuthorizationCodeGrant::get_redirect_url`].
///
/// The last step is to spawn a redirect server and wait for the user
/// to click on the redirect URL in order to extract the access token
/// and the refresh token by calling
/// [`AuthorizationCodeGrant::wait_for_redirection`].
#[derive(Debug)]
pub struct AuthorizationCodeGrant {
    client_id: ClientId,
    client_secret: ClientSecret,
    auth_url: AuthUrl,
    token_url: TokenUrl,
    scopes: Vec<Scope>,
    pkce: Option<(PkceCodeChallenge, PkceCodeVerifier)>,
    redirect_host: String,
    redirect_port: u16,
}

impl AuthorizationCodeGrant {
    /// Create a new Authorization Code Grant with a client ID, a
    /// client secret, an auth URL and a token URL.
    pub fn new<A, B, C, D>(
        client_id: A,
        client_secret: B,
        auth_url: C,
        token_url: D,
    ) -> Result<Self>
    where
        A: ToString,
        B: ToString,
        C: ToString,
        D: ToString,
    {
        Ok(Self {
            client_id: ClientId::new(client_id.to_string()),
            client_secret: ClientSecret::new(client_secret.to_string()),
            auth_url: AuthUrl::new(auth_url.to_string()).map_err(Error::BuildAuthUrlError)?,
            token_url: TokenUrl::new(token_url.to_string()).map_err(Error::BuildTokenUrlError)?,
            scopes: Vec::new(),
            pkce: None,
            redirect_host: String::from("localhost"),
            redirect_port: 9999,
        })
    }

    pub fn with_scope<T>(mut self, scope: T) -> Self
    where
        T: ToString,
    {
        self.scopes.push(Scope::new(scope.to_string()));
        self
    }

    pub fn with_pkce(mut self) -> Self {
        self.pkce = Some(PkceCodeChallenge::new_random_sha256());
        self
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

    /// Build a basic client used to generate the redirect URL and to
    /// exchange the code with an access token and a refresh token.
    pub fn get_client(&self) -> Result<BasicClient> {
        let redirect_uri = RedirectUrl::new(format!(
            "http://{}:{}",
            self.redirect_host, self.redirect_port
        ))
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

    /// Generate the redirect URL using the given client built by
    /// [`AuthorizationCodeGrant::get_client`].
    pub fn get_redirect_url(&self, client: &BasicClient) -> (Url, CsrfToken) {
        let mut url_builder = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(self.scopes.clone());

        if let Some((pkce_challenge, _)) = &self.pkce {
            url_builder = url_builder.set_pkce_challenge(pkce_challenge.clone());
        }

        url_builder.url()
    }

    /// Wait for the user to click on the redirect URL generated by
    /// [`AuthorizationCodeGrant::get_redirect_url`], then exchange
    /// the received code with an access token and a refresh token.
    pub fn wait_for_redirection(
        self,
        client: &BasicClient,
        csrf_state: CsrfToken,
    ) -> Result<(String, Option<String>)> {
        let host = self.redirect_host;
        let port = self.redirect_port;

        // listen for one single connection
        let (mut stream, _) = TcpListener::bind((host.clone(), port))
            .map_err(|err| Error::BindRedirectServerError(host, port, err))?
            .accept()
            .map_err(Error::AcceptRedirectServerError)?;

        // extract the code from the url
        let code = {
            let mut reader = BufReader::new(&stream);

            let mut request_line = String::new();
            reader.read_line(&mut request_line)?;

            let redirect_url = request_line
                .split_whitespace()
                .nth(1)
                .ok_or_else(|| Error::MissingRedirectUrlError(request_line.clone()))?;
            let redirect_url = format!("http://localhost{redirect_url}");
            let redirect_url = Url::parse(&redirect_url)
                .map_err(|err| Error::ParseRedirectUrlError(err, redirect_url.clone()))?;

            let (_, state) = redirect_url
                .query_pairs()
                .find(|(key, _)| key == "state")
                .ok_or_else(|| Error::FindStateInRedirectUrlError(redirect_url.clone()))?;
            let state = CsrfToken::new(state.into_owned());

            if state.secret() != csrf_state.secret() {
                return Err(Error::InvalidStateError(
                    state.secret().to_owned(),
                    csrf_state.secret().to_owned(),
                ));
            }

            let (_, code) = redirect_url
                .query_pairs()
                .find(|(key, _)| key == "code")
                .ok_or_else(|| Error::FindCodeInRedirectUrlError(redirect_url.clone()))?;

            AuthorizationCode::new(code.into_owned())
        };

        // write a basic http response in plain text
        let res = "Authentication successful!";
        let res = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            res.len(),
            res
        );
        stream.write_all(res.as_bytes())?;

        // exchange the code for an access token and a refresh token
        let mut res = client.exchange_code(code);
        if let Some((_, pkce_verifier)) = self.pkce {
            res = res.set_pkce_verifier(pkce_verifier);
        }

        let res = res
            .request(oauth2::reqwest::http_client)
            .map_err(Error::ExchangeCodeError)?;

        let access_token = res.access_token().secret().to_owned();
        let refresh_token = res.refresh_token().map(|t| t.secret().clone());

        Ok((access_token, refresh_token))
    }
}
