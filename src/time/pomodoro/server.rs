use std::{
    io::{self, prelude::*, BufRead, BufReader},
    net::{TcpListener, ToSocketAddrs},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::Request;

pub trait Server {
    fn bind(&self) -> io::Result<()>;
}

pub struct TcpServer {
    listener: TcpListener,
    timer: Arc<Mutex<usize>>,
}

impl TcpServer {
    pub fn new<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(addr)?;
        let timer = Arc::new(Mutex::new(0));
        Ok(Self { listener, timer })
    }
}

impl Server for TcpServer {
    fn bind(&self) -> io::Result<()> {
        for stream in self.listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut reader = BufReader::new(&stream);
                    let mut req = String::new();
                    reader.read_line(&mut req).unwrap();

                    match req.parse() {
                        Ok(Request::Start) => {
                            println!("server: start");

                            *self.timer.lock().unwrap() = 5;
                            let timer = self.timer.clone();

                            thread::spawn(move || loop {
                                {
                                    let mut timer = timer.lock().unwrap();
                                    println!("server timer: {}", *timer);
                                    if *timer == 0 {
                                        break;
                                    }
                                    *timer -= 1;
                                }

                                thread::sleep(Duration::from_secs(1));
                            });
                        }
                        Ok(Request::Get) => {
                            let timer = *self.timer.lock().unwrap();
                            println!("server: get {timer}");
                            stream.write_all(format!("get {timer}\n").as_bytes())?;
                        }
                        Ok(Request::Pause) => {
                            println!("server: pause");
                        }
                        Ok(Request::Resume) => {
                            println!("server: resume");
                        }
                        Ok(Request::Stop) => {
                            println!("server: stop");
                            *self.timer.lock().unwrap() = 0;
                        }
                        Ok(Request::Kill) => {
                            println!("server: kill");
                            *self.timer.lock().unwrap() = 0;
                            break;
                        }
                        Err(err) => {
                            panic!("{err}")
                        }
                    }
                }
                Err(e) => {
                    panic!("{}", e);
                }
            }
        }

        Ok(())
    }
}
