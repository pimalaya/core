use oauth2::{
    basic::BasicClient, reqwest, url::Url, AuthUrl, AuthorizationCode, ClientId, ClientSecret,
    CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
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
}

pub type Result<T> = result::Result<T, Error>;

pub type AccessToken = String;
pub type RefreshToken = Option<String>;

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
            pkce: None,
            scopes: Vec::new(),
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

    pub fn get_redirect_url(&self, client: &BasicClient) -> (Url, CsrfToken) {
        let mut url_builder = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(self.scopes.clone());

        if let Some((pkce_challenge, _)) = &self.pkce {
            url_builder = url_builder.set_pkce_challenge(pkce_challenge.clone());
        }

        url_builder.url()
    }

    pub fn start_redirect_server(
        self,
        client: BasicClient,
        csrf_state: CsrfToken,
    ) -> Result<(AccessToken, RefreshToken)> {
        let host = self.redirect_host;
        let port = self.redirect_port;

        let (mut stream, _) = TcpListener::bind((host.clone(), port))
            .map_err(|err| Error::BindRedirectServerError(host, port, err))?
            .accept()
            .map_err(Error::AcceptRedirectServerError)?;

        let code;
        let state;

        {
            let mut reader = BufReader::new(&stream);

            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();

            let redirect_url = request_line.split_whitespace().nth(1).unwrap();
            let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

            let code_pair = url
                .query_pairs()
                .find(|pair| {
                    let &(ref key, _) = pair;
                    key == "code"
                })
                .unwrap();

            let (_, value) = code_pair;
            code = AuthorizationCode::new(value.into_owned());

            let state_pair = url
                .query_pairs()
                .find(|pair| {
                    let &(ref key, _) = pair;
                    key == "state"
                })
                .unwrap();

            let (_, value) = state_pair;
            state = CsrfToken::new(value.into_owned());
        }

        let message = "Go back to your terminal :)";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
            message.len(),
            message
        );
        stream.write_all(response.as_bytes()).unwrap();

        if state.secret() != csrf_state.secret() {
            return Err(Error::InvalidStateError(
                state.secret().to_owned(),
                csrf_state.secret().to_owned(),
            ));
        }

        let mut builder = client.exchange_code(code);

        if let Some((_, pkce_verifier)) = self.pkce {
            builder = builder.set_pkce_verifier(pkce_verifier);
        }

        let token_response = builder.request(reqwest::http_client);

        let access_token = token_response
            .as_ref()
            .unwrap()
            .access_token()
            .secret()
            .clone();
        // Entry::new("pimalaya-oauth2", "access-token")
        //     .unwrap()
        //     .set_password(&access_token)
        //     .unwrap();

        let refresh_token = token_response
            .as_ref()
            .unwrap()
            .refresh_token()
            .map(|token| token.secret())
            .cloned();
        // if let Some(refresh_token) = &refresh_token {
        //     Entry::new("pimalaya-oauth2", "refresh-token")
        //         .unwrap()
        //         .set_password(refresh_token)
        //         .unwrap();
        // }

        Ok((access_token, refresh_token))
    }
}
