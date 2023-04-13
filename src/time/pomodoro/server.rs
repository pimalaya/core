use log::{debug, error, trace, warn};
use std::{
    io,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::{Request, Response, ThreadSafeTimer, TimerConfig, TimerEvent};

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
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn set_running(&self) -> io::Result<()> {
        self.set(ServerState::Running)
    }

    pub fn set_stopping(&self) -> io::Result<()> {
        self.set(ServerState::Stopping)
    }

    pub fn set_stopped(&self) -> io::Result<()> {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerEvent {
    Started,
    Stopping,
    Stopped,
}

pub type ServerStateChangedHandler =
    Arc<dyn Fn(ServerEvent) -> io::Result<()> + Sync + Send + 'static>;

pub struct ServerConfig {
    handler: ServerStateChangedHandler,
    binders: Vec<Box<dyn ServerBind>>,
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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            handler: Arc::new(|_| Ok(())),
            binders: Vec::new(),
        }
    }
}

#[derive(Default)]
pub struct ServerBuilder {
    server_config: ServerConfig,
    timer_config: TimerConfig,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_server_config(mut self, config: ServerConfig) -> Self {
        self.server_config = config;
        self
    }

    pub fn with_timer_config(mut self, config: TimerConfig) -> Self {
        self.timer_config = config;
        self
    }

    pub fn with_server_handler<H>(mut self, handler: H) -> Self
    where
        H: Fn(ServerEvent) -> io::Result<()> + Sync + Send + 'static,
    {
        self.server_config.handler = Arc::new(handler);
        self
    }

    pub fn with_binder(mut self, binder: Box<dyn ServerBind>) -> Self {
        self.server_config.binders.push(binder);
        self
    }

    pub fn with_timer_handler<H>(mut self, handler: H) -> Self
    where
        H: Fn(TimerEvent) -> io::Result<()> + Sync + Send + 'static,
    {
        self.timer_config.handler = Arc::new(handler);
        self
    }

    pub fn with_work_duration(mut self, duration: usize) -> Self {
        self.timer_config.work_duration = duration;
        self
    }

    pub fn with_short_break_duration(mut self, duration: usize) -> Self {
        self.timer_config.short_break_duration = duration;
        self
    }

    pub fn with_long_break_duration(mut self, duration: usize) -> Self {
        self.timer_config.long_break_duration = duration;
        self
    }

    pub fn build(self) -> Server {
        Server {
            config: self.server_config,
            state: ThreadSafeState::new(),
            timer: ThreadSafeTimer::new(self.timer_config),
        }
    }
}

#[derive(Default)]
pub struct Server {
    config: ServerConfig,
    state: ThreadSafeState,
    timer: ThreadSafeTimer,
}

impl Server {
    pub fn bind(self) -> io::Result<()> {
        self.bind_with(|| loop {
            thread::sleep(Duration::from_secs(1));
        })
    }

    pub fn bind_with(self, wait: impl Fn() -> io::Result<()>) -> io::Result<()> {
        debug!("starting server");

        let fire_event = |event: ServerEvent| {
            if let Err(err) = (self.config.handler)(event.clone()) {
                warn!("cannot fire event {event:?}, skipping it");
                error!("{err}");
            }
        };

        self.state.set_running()?;
        fire_event(ServerEvent::Started);

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

        for binder in self.config.binders {
            let timer = self.timer.clone();
            thread::spawn(move || {
                if let Err(err) = binder.bind(timer) {
                    warn!("cannot bind, exiting");
                    error!("{err}");
                }
            });
        }

        wait()?;

        self.state.set_stopping()?;
        fire_event(ServerEvent::Stopping);

        tick.join()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "cannot wait for timer thread"))?;
        fire_event(ServerEvent::Stopped);

        Ok(())
    }
}
