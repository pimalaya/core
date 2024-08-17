use oauth2::{basic::BasicErrorResponseType, RequestTokenError, StandardErrorResponse};
use std::{io, result};
use thiserror::Error;
use url::Url;

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
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
    IoError(#[from] std::io::Error),

    #[error("cannot refresh access token using the refresh token")]
    RefreshAccessTokenError(
        RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            StandardErrorResponse<BasicErrorResponseType>,
        >,
    ),
}