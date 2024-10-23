use oauth2::{
    basic::BasicErrorResponseType,
    url::{ParseError, Url},
    RequestTokenError, StandardErrorResponse,
};
use thiserror::Error;

/// The global `Result` alias of the module.
pub type Result<T> = std::result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot read response body")]
    ReadResponseBodyError(#[source] http::Error),
    #[error("cannot build auth url")]
    BuildAuthUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot build token url")]
    BuildTokenUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot build revocation url")]
    BuildRevocationUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot build introspection url")]
    BuildIntrospectionUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot build redirect url")]
    BuildRedirectUrlError(#[source] oauth2::url::ParseError),
    #[error("cannot bind redirect server")]
    BindRedirectServerError(String, u16, #[source] std::io::Error),
    #[error("cannot accept redirect server connections")]
    AcceptRedirectServerError(#[source] std::io::Error),
    #[error("invalid state {0}: expected {1}")]
    InvalidStateError(String, String),
    #[error("missing redirect url from {0}")]
    MissingRedirectUrlError(String),
    #[error("cannot parse redirect url {1}")]
    ParseRedirectUrlError(#[source] ParseError, String),
    #[error("cannot find code from redirect url {0}")]
    FindCodeInRedirectUrlError(Url),
    #[error("cannot find state from redirect url {0}")]
    FindStateInRedirectUrlError(Url),
    #[error("cannot exchange code for access and refresh tokens: {0}")]
    ExchangeCodeError(String),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    HttpError(#[from] http::Error),

    #[error("cannot refresh access token using the refresh token")]
    RefreshAccessTokenError(
        Box<RequestTokenError<Error, StandardErrorResponse<BasicErrorResponseType>>>,
    ),
}
