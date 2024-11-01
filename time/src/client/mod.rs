//! # Client
//!
//! The client connects to the server, sends requests in order to
//! control the timer and receive responses.
//!
//! The client must implement the [`Client`] trait.

#[cfg(feature = "tcp-client")]
pub mod tcp;

use std::io::{Error, ErrorKind, Result};

use async_trait::async_trait;
use tracing::{info, trace};

use crate::{
    request::{Request, RequestWriter},
    response::{Response, ResponseReader},
    timer::Timer,
};

/// The client trait.
///
/// Clients must implement this trait. Only the [`Client::send`]
/// function needs to be implemented: it should describe how to
/// connect and send requests to the server.
#[async_trait]
pub trait Client: Send + Sync {
    /// Send the given request and returns the associated response.
    async fn send(&self, req: Request) -> Result<Response>;

    /// Send the start timer request.
    async fn start(&self) -> Result<()> {
        info!("sending request to start timer");

        match self.send(Request::Start).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(Error::new(ErrorKind::Other, err)),
        }
    }

    /// Send the get timer request.
    async fn get(&self) -> Result<Timer> {
        info!("sending request to get timer");

        match self.send(Request::Get).await {
            Ok(Response::Timer(timer)) => {
                trace!("timer: {timer:#?}");
                Ok(timer)
            }
            Ok(res) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(Error::new(ErrorKind::Other, err)),
        }
    }

    /// Send the set timer request.
    async fn set(&self, duration: usize) -> Result<()> {
        info!("sending request to set timer duration");

        match self.send(Request::Set(duration)).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(Error::new(ErrorKind::Other, err)),
        }
    }

    /// Send the pause timer request.
    async fn pause(&self) -> Result<()> {
        info!("sending request to pause timer");

        match self.send(Request::Pause).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(Error::new(ErrorKind::Other, err)),
        }
    }

    /// Send the resume timer request.
    async fn resume(&self) -> Result<()> {
        info!("sending request to resume timer");

        match self.send(Request::Resume).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(Error::new(ErrorKind::Other, err)),
        }
    }

    /// Send the stop timer request.
    async fn stop(&self) -> Result<()> {
        info!("sending request to stop timer");

        match self.send(Request::Stop).await {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(Error::new(ErrorKind::Other, err)),
        }
    }
}

/// The client stream trait.
#[async_trait]
pub trait ClientStream: RequestWriter + ResponseReader {
    async fn handle(&mut self, req: Request) -> Result<Response> {
        self.write(req).await?;
        self.read().await
    }
}

impl<T: RequestWriter + ResponseReader> ClientStream for T {}
