//! # Pomodoro time management module.
//!
//! The [Pomodoro] technique consists of alternating work times
//! (usually 25 min) and break times (5 min or 15 min) in order to
//! maximize efficiency.
//!
//! [Pomodoro]: https://en.wikipedia.org/wiki/Pomodoro_Technique
//!
//! The core concept is the [`Timer`], which gathers information about
//! the cycle (work, short break or long break), the state (running,
//! paused or stopped) and the current timer value (in seconds). The
//! [`Server`] runs the timer and accepts connection from [`Client`]s
//! thanks to [`ServerBind`]ers. The clients communicate with the
//! server using [`Request`]s and [`Response`]s, which allow them to
//! control the timer.
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
//! ```rust
#![doc = include_str!("../examples/pomodoro-tcp.rs")]
//! ```

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
mod clients;
mod request;
mod response;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
mod servers;
mod timer;

#[cfg(feature = "client")]
pub use client::*;
#[cfg(feature = "client")]
pub use clients::*;
pub use request::*;
pub use response::*;
#[cfg(feature = "server")]
pub use server::*;
#[cfg(feature = "server")]
pub use servers::*;
pub use timer::*;
