use log::{error, trace, warn};
use std::{
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

use crate::time::pomodoro::{Request, Response, ThreadSafeTimer};

use super::{Protocol, ProtocolStream};

pub struct Tcp {
    pub host: String,
    pub port: u16,
}

impl ProtocolStream<TcpStream> for Tcp {
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
            Response::Start => String::from("start"),
            Response::Get(timer) => format!("get {}", serde_json::to_string(&timer).unwrap()),
            Response::Stop => String::from("stop"),
            Response::Close => String::from("close"),
        };
        stream.write_all((res + "\n").as_bytes())?;
        Ok(())
    }
}

impl Protocol for Tcp {
    fn bind(&self, timer: ThreadSafeTimer) -> io::Result<()> {
        let listener = TcpListener::bind((self.host.as_str(), self.port))?;

        for stream in listener.incoming() {
            match stream {
                Err(err) => {
                    warn!("skipping invalid listener stream");
                    error!("{err}");
                }
                Ok(mut stream) => {
                    if let Err(err) = self.handle_stream(timer.clone(), &mut stream) {
                        warn!("skipping invalid request");
                        error!("{err}");
                    }
                }
            };
        }

        Ok(())
    }

    fn send(&self, _timer: ThreadSafeTimer) -> io::Result<()> {
        Ok(())
    }
}
