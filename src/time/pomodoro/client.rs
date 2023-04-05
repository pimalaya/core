use log::{info, trace};
use std::io;

use super::{Request, Response, Timer};

pub trait ClientStream<T> {
    fn read(&self, stream: &T) -> io::Result<Response>;
    fn write(&self, stream: &mut T, req: Request) -> io::Result<()>;

    fn handle(&self, stream: &mut T, req: Request) -> io::Result<Response> {
        self.write(stream, req)?;
        self.read(stream)
    }
}

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
