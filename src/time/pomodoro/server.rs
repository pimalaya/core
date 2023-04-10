use log::{debug, error, trace, warn};
use std::{
    io,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::{timer::ThreadSafeTimer, Request, Response};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ServerState {
    Running,
    Stopping,
    #[default]
    Stopped,
}

#[derive(Clone, Debug, Default)]
pub struct ThreadSafeState(Arc<Mutex<ServerState>>);

impl ThreadSafeState {
    fn set(&self, next_state: ServerState) -> io::Result<()> {
        match self.lock() {
            Ok(mut state) => {
                *state = next_state;
                Ok(())
            }
            Err(err) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("cannot lock server state: {err}"),
            )),
        }
    }

    pub fn running(&self) -> io::Result<()> {
        self.set(ServerState::Running)
    }

    pub fn stopping(&self) -> io::Result<()> {
        self.set(ServerState::Stopping)
    }

    pub fn stopped(&self) -> io::Result<()> {
        self.set(ServerState::Stopped)
    }
}

impl Deref for ThreadSafeState {
    type Target = Arc<Mutex<ServerState>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ThreadSafeState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait ServerStream<T> {
    fn read(&self, stream: &T) -> io::Result<Request>;
    fn write(&self, stream: &mut T, res: Response) -> io::Result<()>;

    fn handle(&self, timer: ThreadSafeTimer, stream: &mut T) -> io::Result<()> {
        let req = self.read(stream)?;
        let res = match req {
            Request::Start => {
                debug!("starting timer");
                timer.start()?;
                Response::Ok
            }
            Request::Get => {
                debug!("getting timer");
                let timer = timer.get()?;
                trace!("{timer:#?}");
                Response::Timer(timer)
            }
            Request::Pause => {
                debug!("pausing timer");
                timer.pause()?;
                Response::Ok
            }
            Request::Resume => {
                debug!("resuming timer");
                timer.resume()?;
                Response::Ok
            }
            Request::Stop => {
                debug!("stopping timer");
                timer.stop()?;
                Response::Ok
            }
        };
        self.write(stream, res)?;
        Ok(())
    }
}

pub trait ServerBind: Sync + Send {
    fn bind(&self, timer: ThreadSafeTimer) -> io::Result<()>;
}

#[derive(Default)]
pub struct Server {
    state: ThreadSafeState,
    timer: ThreadSafeTimer,
    binders: Vec<Box<dyn ServerBind>>,
}

impl Server {
    pub fn new<B>(binders: B) -> Self
    where
        B: IntoIterator<Item = Box<dyn ServerBind>>,
    {
        Self {
            binders: binders.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn bind(self) -> io::Result<()> {
        self.bind_with(|| loop {
            thread::sleep(Duration::from_secs(1));
        })
    }

    pub fn bind_with(self, wait: impl Fn() -> io::Result<()>) -> io::Result<()> {
        debug!("starting server");

        self.state.running()?;

        let state = self.state.clone();
        let timer = self.timer.clone();
        let tick = thread::spawn(move || {
            for timer in timer {
                match state.lock() {
                    Ok(mut state) => match *state {
                        ServerState::Stopping => {
                            *state = ServerState::Stopped;
                            break;
                        }
                        ServerState::Stopped => {
                            break;
                        }
                        ServerState::Running => {
                            // sleep 1s outside of the lock
                        }
                    },
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

        thread::spawn(move || {
            for binder in self.binders {
                if let Err(err) = binder.bind(self.timer.clone()) {
                    warn!("cannot bind, exiting");
                    error!("{err}");
                }
            }
        });

        wait()?;

        self.state.stopping()?;

        tick.join()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "cannot wait for timer thread"))
    }
}
