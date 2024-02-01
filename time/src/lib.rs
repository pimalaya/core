//! # ⏳ time-lib
//!
//! Rust library to manange time.
//!
//! The core concept is the [`Timer`], which contains information
//! about the cycle and the state. The [`Server`] runs the timer and
//! accepts connection from [`Client`]s using [`ServerBind`]ers. The
//! clients communicate with the server using [`Request`]s and
//! [`Response`]s, which allow them to control the timer. The timer
//! can be customized using [`TimerConfig`] and [`TimerCycle`].
//!
//! ```text,ignore
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
//! Example using TCP and the [Pomodoro] technique:
//!
//! ```shell,ignore
//! $ cargo run --example pomodoro-tcp
//! ```
//!
//! ```rust,ignore
#![doc = include_str!("../examples/pomodoro-tcp.rs")]
//! ```
//!
//! See [more examples].
//!
//! [Pomodoro]: https://en.wikipedia.org/wiki/Pomodoro_Technique
//! [more examples]: https://git.sr.ht/~soywod/pimalaya/tree/master/item/time/examples

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

#[cfg(feature = "client")]
pub mod client;
pub mod request;
pub mod response;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "tcp-any")]
pub mod tcp;
pub mod timer;
