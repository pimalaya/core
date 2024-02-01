//! # Client module.
//!
//! The client connects to the server, sends requests in order to
//! control the timer and receive responses.
//!
//! A client must implement the [`Client`] trait. A client may
//! implement the [`ClientStream`] trait as well in order to reduce
//! the complexity of the [`Client`]'s implementation.

#[cfg(feature = "tcp-client")]
mod tcp;

use async_trait::async_trait;
use log::{info, trace};
use std::io;

use crate::{RequestWriter, ResponseReader};

use super::{Request, Response, Timer};

#[cfg(feature = "tcp-client")]
pub use self::tcp::*;

/// The client trait.
///
/// Clients must implement this trait. Only the [`Client::send`]
/// function needs to be implemented: it should describe how to
/// connect and send requests to the server.
#[async_trait]
pub trait Client: Send + Sync {
    async fn send(&self, req: Request) -> io::Result<Response>;

    async fn start(&self) -> io::Result<()> {
        info!("sending request to start timer");

        match self.send(Request::Start).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    async fn get(&self) -> io::Result<Timer> {
        info!("sending request to get timer");

        match self.send(Request::Get).await {
            Ok(Response::Timer(timer)) => {
                trace!("timer: {timer:#?}");
                Ok(timer)
            }
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    async fn set(&self, duration: usize) -> io::Result<()> {
        info!("sending request to set timer duration");

        match self.send(Request::Set(duration)).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    async fn pause(&self) -> io::Result<()> {
        info!("sending request to pause timer");

        match self.send(Request::Pause).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    async fn resume(&self) -> io::Result<()> {
        info!("sending request to resume timer");

        match self.send(Request::Resume).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    async fn stop(&self) -> io::Result<()> {
        info!("sending request to stop timer");

        match self.send(Request::Stop).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }
}

/// The client stream trait.
///
/// Clients may implement this trait, but it is not mandatory. It can
/// be seen as a helper: by implementing the [`ClientStream::read`]
/// and the [`ClientStream::write`] functions, the trait can deduce
/// how to handle a request.
#[async_trait]
pub trait ClientStream: RequestWriter + ResponseReader {
    async fn handle(&mut self, req: Request) -> io::Result<Response> {
        self.write(req).await?;
        self.read().await
    }
}

impl<T: RequestWriter + ResponseReader> ClientStream for T {}
