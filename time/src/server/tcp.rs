//! # TCP binder
//!
//! This module contains the implementation of the TCP server binder,
//! based on [`tokio::net::TcpStream`].

use std::io;

#[cfg(feature = "async-std")]
use async_std::net::TcpListener;
use async_trait::async_trait;
use futures::{AsyncBufReadExt, AsyncWriteExt};
#[cfg(feature = "tokio")]
use tokio::net::TcpListener;
use tracing::debug;

use crate::{
    request::{Request, RequestReader},
    response::{Response, ResponseWriter},
    tcp::TcpHandler,
    timer::ThreadSafeTimer,
};

use super::{ServerBind, ServerStream};

/// The TCP server binder.
///
/// This [`ServerBind`]er uses the TCP protocol to bind a listener, to
/// read requests and write responses.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TcpBind {
    /// The TCP host of the listener.
    pub host: String,

    /// The TCP port of the listener.
    pub port: u16,
}

impl TcpBind {
    /// Create a new TCP binder using the given host and port.
    pub fn new(host: impl ToString, port: u16) -> Box<dyn ServerBind> {
        Box::new(Self {
            host: host.to_string(),
            port,
        })
    }
}

#[async_trait]
impl ServerBind for TcpBind {
    async fn bind(&self, timer: ThreadSafeTimer) -> io::Result<()> {
        let listener = TcpListener::bind((self.host.as_str(), self.port)).await?;

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    debug!("TCP connection accepted");

                    let mut handler = TcpHandler::new(stream);
                    if let Err(err) = handler.handle(timer.clone()).await {
                        debug!("cannot handle request");
                        debug!("{err:?}");
                    }
                }
                Err(err) => {
                    debug!("cannot get stream from client");
                    debug!("{err:?}");
                }
            }
        }
    }
}

#[async_trait]
impl RequestReader for TcpHandler {
    async fn read(&mut self) -> io::Result<Request> {
        let mut req = String::new();
        self.reader.read_line(&mut req).await?;

        let mut tokens = req.split_whitespace();
        match tokens.next() {
            Some("start") => Ok(Request::Start),
            Some("get") => Ok(Request::Get),
            Some("set") => match tokens.next().map(|duration| duration.parse::<usize>()) {
                Some(Ok(duration)) => Ok(Request::Set(duration)),
                Some(Err(err)) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("invalid duration: {err}"),
                )),
                None => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "missing duration".to_owned(),
                )),
            },
            Some("pause") => Ok(Request::Pause),
            Some("resume") => Ok(Request::Resume),
            Some("stop") => Ok(Request::Stop),
            Some(req) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid request: {req}"),
            )),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "missing request".to_owned(),
            )),
        }
    }
}

#[async_trait]
impl ResponseWriter for TcpHandler {
    async fn write(&mut self, res: Response) -> io::Result<()> {
        let res = match res {
            Response::Ok => "ok\n".to_string(),
            Response::Timer(timer) => {
                format!("timer {}\n", serde_json::to_string(&timer).unwrap())
            }
        };

        self.writer.write_all(res.as_bytes()).await?;

        Ok(())
    }
}
