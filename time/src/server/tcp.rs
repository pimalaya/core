//! # TCP server binder module.
//!
//! This module contains the implementation of the TCP server binder,
//! based on [`std::net::TcpStream`].

use log::{error, trace, warn};
use std::{
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

use crate::{Request, Response, ServerBind, ServerStream, ThreadSafeTimer};

pub struct TcpBind {
    pub host: String,
    pub port: u16,
}

impl TcpBind {
    pub fn new<H>(host: H, port: u16) -> Box<dyn ServerBind>
    where
        H: ToString,
    {
        Box::new(Self {
            host: host.to_string(),
            port,
        })
    }
}

impl ServerStream<TcpStream> for TcpBind {
    /// Read the given [`std::net::TcpStream`] to extract the request
    /// sent by the client.
    fn read(&self, stream: &TcpStream) -> io::Result<Request> {
        let mut reader = BufReader::new(stream);
        let mut req = String::new();
        reader.read_line(&mut req).unwrap();

        trace!("request: {req:?}");

        let mut tokens = req.trim().split_whitespace();
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
                    format!("missing duration"),
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
                format!("missing request"),
            )),
        }
    }

    /// Write the given response to the given [`std::net::TcpStream`].
    fn write(&self, stream: &mut TcpStream, res: Response) -> io::Result<()> {
        let res = match res {
            Response::Ok => String::from("ok"),
            Response::Timer(timer) => format!("timer {}", serde_json::to_string(&timer).unwrap()),
        };
        stream.write_all((res + "\n").as_bytes())?;
        Ok(())
    }
}

impl ServerBind for TcpBind {
    /// To bind, the [`TcpBind`] gets a [`std::net::TcpListener`] then
    /// indefinitely waits for incoming requests. When a connection
    /// comes, [`TcpBind`] retrieves the associated
    /// [`std::net::TcpStream`] and send it to the helper
    /// [`crate::ServerStream::handle`].
    fn bind(&self, timer: ThreadSafeTimer) -> io::Result<()> {
        let binder = TcpListener::bind((self.host.as_str(), self.port))?;

        for stream in binder.incoming() {
            match stream {
                Err(err) => {
                    warn!("skipping invalid listener stream");
                    error!("{err}");
                }
                Ok(mut stream) => {
                    if let Err(err) = self.handle(timer.clone(), &mut stream) {
                        warn!("skipping invalid request");
                        error!("{err}");
                    }
                }
            };
        }

        Ok(())
    }
}
