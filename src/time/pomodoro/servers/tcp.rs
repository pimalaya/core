use log::{error, trace, warn};
use std::{
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

use crate::time::pomodoro::{Request, Response, ServerBind, ServerStream, ThreadSafeTimer};

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
    fn read(&self, stream: &TcpStream) -> io::Result<Request> {
        let mut reader = BufReader::new(stream);
        let mut req = String::new();
        reader.read_line(&mut req).unwrap();

        trace!("request: {req:?}");

        match req.trim() {
            "start" => Ok(Request::Start),
            "get" => Ok(Request::Get),
            "pause" => Ok(Request::Pause),
            "resume" => Ok(Request::Resume),
            "stop" => Ok(Request::Stop),
            unknown => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid request: {unknown}"),
            )),
        }
    }

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
