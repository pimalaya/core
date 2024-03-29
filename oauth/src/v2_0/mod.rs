//! This module provides helpers to simplify OAuth 2.0 flows, based on
//! the [RFC6749](https://datatracker.ietf.org/doc/html/rfc6749).
//!
//! ```rust,ignore
#![doc = include_str!("../../examples/gmail.rs")]
//! ```

pub mod authorization_code_grant;
pub mod client;
pub mod refresh_access_token;

pub use authorization_code_grant::AuthorizationCodeGrant;
pub use client::Client;
pub use refresh_access_token::RefreshAccessToken;
