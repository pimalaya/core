use log::{info, trace};
use std::{io, net::TcpStream};

use super::{request::Request, response::Response, timer::Timer};

pub trait Client {
    type Handler;

    fn connect(&self) -> io::Result<Self::Handler>;
    fn read(&self, handler: &Self::Handler) -> io::Result<Response>;
    fn write(&mut self, handler: &mut Self::Handler, req: Request) -> io::Result<()>;

    fn start(&mut self) -> io::Result<()> {
        info!("sending request to start timer");

        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Start)
    }

    fn get(&mut self) -> io::Result<Timer> {
        info!("sending request to get timer");

        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Get)?;
        match self.read(&handler) {
            Ok(Response::Get(timer)) => {
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

    fn pause(&mut self) -> io::Result<()> {
        info!("sending request to pause timer");

        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Pause)
    }

    fn resume(&mut self) -> io::Result<()> {
        info!("sending request to resume timer");

        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Resume)
    }

    fn stop(&mut self) -> io::Result<()> {
        info!("sending request to stop timer");

        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Stop)
    }
}

pub struct TcpClient {
    addr: String,
}

impl TcpClient {
    pub fn new<A>(addr: A) -> Self
    where
        A: ToString,
    {
        let addr = addr.to_string();
        Self { addr }
    }
}

impl Client for TcpClient {
    type Handler = TcpStream;

    fn connect(&self) -> io::Result<Self::Handler> {
        TcpStream::connect(&self.addr)
    }

    fn read(&self, _handler: &TcpStream) -> io::Result<Response> {
        Ok(Response::Ok)
    }

    fn write(&mut self, _handler: &mut TcpStream, _req: Request) -> io::Result<()> {
        Ok(())
    }
}
