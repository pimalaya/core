//! # pimalaya-oauth2
//!
//! This crate provides helpers to simplify OAuth 2.0 flows, based on
//! the [RFC6749](https://datatracker.ietf.org/doc/html/rfc6749).
//!
//! ```rust,ignore
#![doc = include_str!("../examples/gmail.rs")]
//! ```

pub mod authorization_code_grant;
pub mod refresh_access_token;

pub use authorization_code_grant::AuthorizationCodeGrant;
pub use refresh_access_token::RefreshAccessToken;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    AuthorizationCodeGrant(#[from] authorization_code_grant::Error),
    #[error(transparent)]
    RefreshAccessToken(#[from] refresh_access_token::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
