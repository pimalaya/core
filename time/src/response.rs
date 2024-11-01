//! # Response
//!
//! When a server receives a request, it sends back a response. This
//! module contains the response structure as well as traits to read
//! and write a response.

use std::io::Result;

use async_trait::async_trait;

use crate::timer::Timer;

/// The server response struct.
///
/// Responses are sent by servers and received by clients.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    /// Default response when everything goes as expected.
    Ok,

    /// Response containing the current timer.
    Timer(Timer),
}

/// Trait to read a server response.
///
/// Describes how a response should be parsed by a client.
#[async_trait]
pub trait ResponseReader: Send + Sync {
    /// Read the current server response.
    async fn read(&mut self) -> Result<Response>;
}

/// Trait to write a response.
///
/// Describes how a response should be sent by a server.
#[async_trait]
pub trait ResponseWriter: Send + Sync {
    /// Write the given response.
    async fn write(&mut self, res: Response) -> Result<()>;
}
