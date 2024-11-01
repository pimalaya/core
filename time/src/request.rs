//! # Request
//!
//! To control the timer, a client sends requests to the server and
//! receive back a response. This module contains the request
//! structure as well as trait to read and write a request.

use std::io::Result;

use async_trait::async_trait;

/// The client request struct.
///
/// Requests are sent by clients and received by servers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    /// Request the timer to start with the first configured cycle.
    Start,

    /// Request the state, the cycle and the value of the timer.
    Get,

    /// Request to change the current timer duration.
    Set(usize),

    /// Request to pause the timer.
    ///
    /// A paused timer freezes, which means it keeps its state, cycle
    /// and value till it get resumed.
    Pause,

    /// Request to resume the paused timer.
    ///
    /// Has no effect if the timer is not paused.
    Resume,

    /// Request to stop the timer.
    ///
    /// Stopping the timer resets the state, the cycle and the value.
    Stop,
}

/// Trait to read a client request.
///
/// Describes how a request should be parsed by a server.
#[async_trait]
pub trait RequestReader: Send + Sync {
    /// Read the current client request.
    async fn read(&mut self) -> Result<Request>;
}

/// Trait to write a client request.
///
/// Describes how a request should be sent by a client.
#[async_trait]
pub trait RequestWriter: Send + Sync {
    /// Write the given client request.
    async fn write(&mut self, req: Request) -> Result<()>;
}
