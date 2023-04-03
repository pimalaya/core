use log::{debug, error, trace, warn};
use std::{
    io::{self, prelude::*, BufRead, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::{request::Request, timer::ThreadSafeTimer};

pub trait Server {
    fn start(&self) -> io::Result<thread::JoinHandle<()>>;
    fn stop(&self) -> io::Result<()>;
}

pub struct TcpServer {
    addr: String,
    timer: ThreadSafeTimer,
    should_stop: Arc<Mutex<bool>>,
}

impl TcpServer {
    pub fn new<A>(addr: A) -> Self
    where
        A: ToString,
    {
        Self {
            addr: addr.to_string(),
            timer: ThreadSafeTimer::new(),
            should_stop: Arc::new(Mutex::new(false)),
        }
    }
}

impl Server for TcpServer {
    fn start(&self) -> io::Result<thread::JoinHandle<()>> {
        debug!("starting server");

        let timer = self.timer.clone();
        let should_stop = self.should_stop.clone();
        let tick = thread::spawn(move || {
            for timer in timer {
                match should_stop.lock().map(|guard| *guard) {
                    Ok(true) => break,
                    Ok(false) => {}
                    Err(err) => {
                        warn!("cannot determine if server should stop, exiting");
                        error!("{err}");
                        break;
                    }
                }
                trace!("timer tick: {timer:#?}");
                thread::sleep(Duration::from_secs(1));
            }
        });

        let timer = self.timer.clone();
        let listener = TcpListener::bind(&self.addr)?;
        let handle_stream = move |mut stream: TcpStream| -> io::Result<()> {
            let mut reader = BufReader::new(&stream);
            let mut req = String::new();
            reader.read_line(&mut req).unwrap();

            trace!("request: {req:?}");

            match req.parse() {
                Ok(Request::Start) => {
                    debug!("starting timer");
                    timer.start()
                }
                Ok(Request::Get) => {
                    debug!("getting timer");

                    let timer = timer.get()?;
                    trace!("{timer:#?}");

                    let res = format!("get {}\n", serde_json::to_string(&timer).unwrap());
                    trace!("response: {res:?}");
                    stream.write_all(res.as_bytes())
                }
                Ok(Request::Pause) => {
                    debug!("pausing timer");
                    timer.pause()
                }
                Ok(Request::Resume) => {
                    debug!("resuming timer");
                    timer.resume()
                }
                Ok(Request::Stop) => {
                    debug!("stopping timer");
                    timer.stop()
                }
                Err(err) => Err(err),
            }
        };

        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Err(err) => {
                        warn!("skipping invalid listener stream");
                        error!("{err}");
                    }
                    Ok(stream) => {
                        if let Err(err) = handle_stream(stream) {
                            warn!("skipping invalid request");
                            error!("{err}");
                        }
                    }
                };
            }
        });

        Ok(tick)
    }

    fn stop(&self) -> io::Result<()> {
        debug!("stopping server");

        let mut should_stop = self
            .should_stop
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        *should_stop = true;

        Ok(())
    }
}
