//! This module provides helpers to simplify OAuth 2.0 flows, based on
//! the [RFC6749](https://datatracker.ietf.org/doc/html/rfc6749).
//!
//! ```rust,ignore
#![doc = include_str!("../../examples/gmail.rs")]
//! ```

mod authorization_code_grant;
mod client;
mod error;
mod refresh_access_token;

#[doc(inline)]
pub use self::{
    authorization_code_grant::AuthorizationCodeGrant,
    client::Client,
    error::{Error, Result},
    refresh_access_token::RefreshAccessToken,
};
