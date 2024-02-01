//! # Request module.
//!
//! A [`Request`] is the type of data sent by the client to the server
//! in order to control the timer.

use std::io;

use async_trait::async_trait;

/// The request struct.
///
/// Request are sent by clients and received by servers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    /// Request the timer to start with the first work cycle.
    Start,

    /// Request the state, the cycle and the value of the timer.
    Get,

    /// Request to change the current timer duration.
    Set(usize),

    /// Request to pause the timer. A paused timer freezes, which
    /// means it keeps its state, cycle and value till it get resumed.
    Pause,

    /// Request to resume the paused timer.
    Resume,

    /// Request to stop the timer. Stopping the timer resets the
    /// state, cycle and the value.
    Stop,
}

#[async_trait]
pub trait RequestReader: Send + Sync {
    async fn read(&mut self) -> io::Result<Request>;
}

#[async_trait]
pub trait RequestWriter: Send + Sync {
    async fn write(&mut self, req: Request) -> io::Result<()>;
}
