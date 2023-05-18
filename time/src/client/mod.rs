//! # Client module.
//!
//! The client connects to the server, sends requests in order to
//! control the timer and returns responses.
//!
//! A client must implement the [`Client`] trait. A client may
//! implement the [`ClientStream`] trait as well in order to reduce
//! the complexity of the [`Client`]'s implementation.

#[cfg(feature = "tcp-client")]
mod tcp;
#[cfg(feature = "tcp-client")]
pub use tcp::*;

use log::{info, trace};
use std::io;

use super::{Request, Response, Timer};

/// Clients must implement this trait. Only the [`Client::send`]
/// function needs to be implemented: it should describe how to
/// connect and send requests to the server.
pub trait Client {
    fn send(&self, req: Request) -> io::Result<Response>;

    fn start(&self) -> io::Result<()> {
        info!("sending request to start timer");

        match self.send(Request::Start) {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    fn get(&self) -> io::Result<Timer> {
        info!("sending request to get timer");

        match self.send(Request::Get) {
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

    fn set(&self, duration: usize) -> io::Result<()> {
        info!("sending request to set timer duration");

        match self.send(Request::Set(duration)) {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    fn pause(&self) -> io::Result<()> {
        info!("sending request to pause timer");

        match self.send(Request::Pause) {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    fn resume(&self) -> io::Result<()> {
        info!("sending request to resume timer");

        match self.send(Request::Resume) {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }

    fn stop(&self) -> io::Result<()> {
        info!("sending request to stop timer");

        match self.send(Request::Stop) {
            Ok(Response::Ok) => Ok(()),
            Ok(res) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid response: {res:?}"),
            )),
            Err(err) => Err(io::Error::new(io::ErrorKind::Other, err)),
        }
    }
}

/// Clients may implement this trait, but it is not mandatory. It can
/// be seen as a helper: by implementing the [`ClientStream::read`]
/// and the [`ClientStream::write`] functions, the trait can deduce
/// how to handle a request.
pub trait ClientStream<T> {
    fn read(&self, stream: &T) -> io::Result<Response>;
    fn write(&self, stream: &mut T, req: Request) -> io::Result<()>;

    fn handle(&self, stream: &mut T, req: Request) -> io::Result<Response> {
        self.write(stream, req)?;
        self.read(stream)
    }
}
