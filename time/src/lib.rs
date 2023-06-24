//! # Pimalaya time management
//!
//! Rust library to manange your time.
//!
//! The core concept is the [`Timer`], which gathers information about
//! the cycle and the state. The [`Server`] runs the timer and accepts
//! connection from [`Client`]s thanks to [`ServerBind`]ers. The
//! clients communicate with the server using [`Request`]s and
//! [`Response`]s, which allow them to control the timer.
//!
//! ```ignore
//! ┌────────────────────────┐
//! │Server                  │
//! │             ┌────────┐ │ Request ┌────────┐
//! │             │        │◄├─────────┤        │
//! │    ┌────────┤Binder A│ │         │Client A│
//! │    │        │        ├─┼────────►│        │
//! │    │        └────────┘ │Response └────────┘
//! │    │                   │
//! │    ▼        ┌────────┐ │         ┌────────┐
//! │ ┌─────┐     │        │◄├─────────┤        │
//! │ │Timer│◄────┤Binder B│ │         │Client B│
//! │ └─────┘     │        ├─┼────────►│        │
//! │    ▲        └────────┘ │         └────────┘
//! │    │                   │
//! │    │        ┌────────┐ │         ┌────────┐
//! │    │        │        │◄├─────────┤        │
//! │    └────────┤Binder C│ │         │Client C│
//! │             │        ├─┼────────►│        │
//! │             └────────┘ │         └────────┘
//! │                        │
//! └────────────────────────┘
//! ```
//!
//! ```rust,ignore
#![doc = include_str!("../examples/pomodoro-tcp.rs")]
//! ```

#[cfg(feature = "client")]
mod client;
mod request;
mod response;
#[cfg(feature = "server")]
mod server;
mod timer;

#[cfg(feature = "client")]
pub use client::*;
pub use request::*;
pub use response::*;
#[cfg(feature = "server")]
pub use server::*;
pub use timer::*;
