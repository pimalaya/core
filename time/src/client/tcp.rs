//! # TCP client module.
//!
//! This module contains the implementation of the TCP client, based
//! on [`std::net::TcpStream`].

use async_trait::async_trait;
use std::io;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    tcp::TcpHandler, Client, ClientStream, Request, RequestWriter, Response, ResponseReader, Timer,
};

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
    pub fn new(host: impl ToString, port: u16) -> Box<dyn Client> {
        Box::new(Self {
            host: host.to_string(),
            port,
        })
    }
}

#[async_trait]
impl Client for TcpClient {
    /// Send the given request to the TCP server.
    async fn send(&self, req: Request) -> io::Result<Response> {
        let stream = TcpStream::connect((self.host.as_str(), self.port)).await?;
        let mut handler = TcpHandler::from(stream);
        handler.handle(req).await
    }
}

#[async_trait]
impl RequestWriter for TcpHandler {
    async fn write(&mut self, req: Request) -> io::Result<()> {
        let req = match req {
            Request::Start => format!("start\n"),
            Request::Get => format!("get\n"),
            Request::Set(duration) => format!("set {duration}\n"),
            Request::Pause => format!("pause\n"),
            Request::Resume => format!("resume\n"),
            Request::Stop => format!("stop\n"),
        };

        self.writer.write_all(req.as_bytes()).await?;

        Ok(())
    }
}

#[async_trait]
impl ResponseReader for TcpHandler {
    async fn read(&mut self) -> io::Result<Response> {
        let mut res = String::new();
        self.reader.read_line(&mut res).await?;

        let mut tokens = res.split_whitespace();
        match tokens.next() {
            Some("ok") => Ok(Response::Ok),
            Some("timer") => match tokens.next().map(serde_json::from_str::<Timer>) {
                Some(Ok(timer)) => Ok(Response::Timer(timer)),
                Some(Err(err)) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid timer: {err}"),
                )),
                None => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing timer".to_owned(),
                )),
            },
            Some(res) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid response: {res}"),
            )),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "missing response".to_owned(),
            )),
        }
    }
}
