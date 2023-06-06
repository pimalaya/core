//! # pimalaya-oauth2
//!
//! This crate provides helpers to simplify OAuth 2.0 flows, based on
//! the [RFC6749](https://datatracker.ietf.org/doc/html/rfc6749).
//!
//! ```rust,ignore
#![doc = include_str!("../examples/gmail.rs")]
//! ```

pub mod authorization_code_grant;
pub mod client;
pub mod refresh_access_token;

pub use authorization_code_grant::AuthorizationCodeGrant;
pub use client::Client;
pub use refresh_access_token::RefreshAccessToken;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ClientError(#[from] client::Error),
    #[error(transparent)]
    AuthorizationCodeGrantError(#[from] authorization_code_grant::Error),
    #[error(transparent)]
    RefreshAccessTokenError(#[from] refresh_access_token::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
