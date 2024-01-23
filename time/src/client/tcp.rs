//! # TCP client module.
//!
//! This module contains the implementation of the TCP client, based
//! on [`std::net::TcpStream`].

use log::trace;
use std::{
    io::{self, BufRead, BufReader, Write},
    net::TcpStream,
};

use crate::{Client, ClientStream, Request, Response, Timer};

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

impl ClientStream<TcpStream> for TcpClient {
    /// Read the given [`std::net::TcpStream`] to extract the response
    /// sent by the server.
    fn read(&self, stream: &TcpStream) -> io::Result<Response> {
        let mut reader = BufReader::new(stream);
        let mut res = String::new();
        reader.read_line(&mut res).unwrap();

        trace!("receiving response: {res:?}");

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

    /// Write the given request to the given [`std::net::TcpStream`].
    fn write(&self, stream: &mut TcpStream, req: Request) -> io::Result<()> {
        trace!("sending request: {req:?}");

        let req = match req {
            Request::Start => String::from("start"),
            Request::Get => String::from("get"),
            Request::Set(duration) => format!("set {duration}"),
            Request::Pause => String::from("pause"),
            Request::Resume => String::from("resume"),
            Request::Stop => String::from("stop"),
        };
        stream.write_all((req + "\n").as_bytes())?;
        Ok(())
    }
}

impl Client for TcpClient {
    /// Send the given request to the TCP server.
    ///
    /// To send a request, the [`TcpClient`] retrieves the
    /// [`std::net::TcpStream`] by connecting to the server, then
    /// handles it using the helper [`crate::ClientStream::handle`].
    fn send(&self, req: Request) -> io::Result<Response> {
        let mut stream = TcpStream::connect((self.host.as_str(), self.port))?;
        self.handle(&mut stream, req)
    }
}
