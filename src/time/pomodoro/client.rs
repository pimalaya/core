use std::{
    io::{self, prelude::*, BufReader},
    net::TcpStream,
};

use super::{Request, Response};

pub trait Client {
    type Handler;

    fn connect(&self) -> io::Result<Self::Handler>;
    fn read(&self, handler: &Self::Handler) -> io::Result<Response>;
    fn write(&mut self, handler: &mut Self::Handler, req: Request) -> io::Result<()>;

    fn start(&mut self) -> io::Result<()> {
        println!("client: start");
        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Start)
    }

    fn get(&mut self) -> io::Result<usize> {
        println!("client: get");
        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Get)?;
        match self.read(&handler) {
            Ok(Response::Get(timer)) => {
                println!("client timer: {timer}");
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
        println!("client: pause");
        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Pause)
    }

    fn resume(&mut self) -> io::Result<()> {
        println!("client: resume");
        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Resume)
    }

    fn stop(&mut self) -> io::Result<()> {
        println!("client: stop");
        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Stop)
    }

    fn kill(&mut self) -> io::Result<()> {
        println!("client: kill");
        let mut handler = self.connect()?;
        self.write(&mut handler, Request::Kill)
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

    fn read(&self, handler: &TcpStream) -> io::Result<Response> {
        let mut reader = BufReader::new(handler);
        let mut res = String::new();
        reader.read_line(&mut res).unwrap();
        res.parse()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    fn write(&mut self, handler: &mut TcpStream, req: Request) -> io::Result<()> {
        handler.write_all((req.to_string() + "\n").as_bytes())?;
        Ok(())
    }
}
