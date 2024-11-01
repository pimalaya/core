//! # TCP client
//!
//! This module contains the implementation of the TCP client, based
//! on [`tokio::net::TcpStream`].

use std::io::{Error, ErrorKind, Result};

use async_trait::async_trait;
use futures::{AsyncBufReadExt, AsyncWriteExt};
use tracing::debug;

use crate::{
    request::{Request, RequestWriter},
    response::{Response, ResponseReader},
    tcp::{TcpHandler, TcpStream},
    timer::Timer,
};

use super::{Client, ClientStream};

/// The TCP client.
///
/// This [`Client`] uses the TCP protocol to connect to a listener, to
/// read responses and write requests.
pub struct TcpClient {
    /// The TCP host the client should connect to.
    pub host: String,

    /// The TCP port the client should connect to.
    pub port: u16,
}

impl TcpClient {
    /// Create a new TCP client using the given host and port.
    pub fn new_boxed(host: impl ToString, port: u16) -> Box<dyn Client> {
        Box::new(Self {
            host: host.to_string(),
            port,
        })
    }
}

#[async_trait]
impl Client for TcpClient {
    /// Send the given request to the TCP server.
    async fn send(&self, req: Request) -> Result<Response> {
        debug!("TCP connection accepted");
        let stream = TcpStream::connect((self.host.as_str(), self.port)).await?;
        let mut handler = TcpHandler::new(stream);
        handler.handle(req).await
    }
}

#[async_trait]
impl RequestWriter for TcpHandler {
    async fn write(&mut self, req: Request) -> Result<()> {
        let req = match req {
            Request::Start => "start\n".to_owned(),
            Request::Get => "get\n".to_owned(),
            Request::Set(duration) => format!("set {duration}\n"),
            Request::Pause => "pause\n".to_owned(),
            Request::Resume => "resume\n".to_owned(),
            Request::Stop => "stop\n".to_owned(),
        };

        self.writer.write_all(req.as_bytes()).await?;

        Ok(())
    }
}

#[async_trait]
impl ResponseReader for TcpHandler {
    async fn read(&mut self) -> Result<Response> {
        let mut res = String::new();
        self.reader.read_line(&mut res).await?;

        let mut tokens = res.split_whitespace();
        match tokens.next() {
            Some("ok") => Ok(Response::Ok),
            Some("timer") => match tokens.next().map(serde_json::from_str::<Timer>) {
                Some(Ok(timer)) => Ok(Response::Timer(timer)),
                Some(Err(err)) => Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("invalid timer: {err}"),
                )),
                None => Err(Error::new(
                    ErrorKind::InvalidInput,
                    "missing timer".to_owned(),
                )),
            },
            Some(res) => Err(Error::new(
                ErrorKind::InvalidInput,
                format!("invalid response: {res}"),
            )),
            None => Err(Error::new(
                ErrorKind::InvalidInput,
                "missing response".to_owned(),
            )),
        }
    }
}
