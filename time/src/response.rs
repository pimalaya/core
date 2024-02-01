//! # Response module.
//!
//! A [`Response`] is the type of data sent by the server to the
//! client straight after receiving a request.

use async_trait::async_trait;
use std::io;

use super::Timer;

/// The response struct.
///
/// Responses are sent by servers and received by clients.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    /// Default response when everything goes fine.
    Ok,

    /// Response that contains the current timer.
    Timer(Timer),
}

#[async_trait]
pub trait ResponseReader: Send + Sync {
    async fn read(&mut self) -> io::Result<Response>;
}

#[async_trait]
pub trait ResponseWriter: Send + Sync {
    async fn write(&mut self, res: Response) -> io::Result<()>;
}
