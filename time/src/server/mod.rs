//! # Server module.
//!
//! The [`Server`] runs the timer, accepts connections from clients
//! and sends responses. The [`Server`] accepts connections using
//! [`ServerBind`]ers. The [`Server`] should have at least one
//! [`ServerBind`], otherwise it stops by itself.

#[cfg(feature = "tcp-binder")]
mod tcp;
#[cfg(feature = "tcp-binder")]
pub use tcp::*;

use log::{debug, error, trace, warn};
use std::{
    io,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{TimerCycle, TimerLoop};

use super::{Request, Response, ThreadSafeTimer, TimerConfig, TimerEvent};

/// The server state enum.
///
/// This enum represents the different states the server can be in.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ServerState {
    /// The server is in running mode, which blocks the main process.
    Running,
    /// The server received the order to stop.
    Stopping,
    /// The server is stopped and will free the main process.
    #[default]
    Stopped,
}

/// The server configuration.
pub struct ServerConfig {
    /// The server state change handler.
    handler: ServerStateChangedHandler,

    /// The binders list the server should use when starting up.
    binders: Vec<Box<dyn ServerBind>>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            handler: Arc::new(|_| Ok(())),
            binders: Vec::new(),
        }
    }
}

/// The server state changed event.
///
/// Event triggered by [`ServerStateChangedHandler`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerEvent {
    Started,
    Stopping,
    Stopped,
}

/// The server state changed handler alias.
pub type ServerStateChangedHandler = Arc<dyn Fn(ServerEvent) -> io::Result<()> + Sync + Send>;

/// Thread safe version of the [`ServerState`].
///
/// It allows the [`Server`] to mutate its state even from a
/// [`std::thread::spawn`]).
#[derive(Clone, Debug, Default)]
pub struct ThreadSafeState(Arc<Mutex<ServerState>>);

impl ThreadSafeState {
    /// Create a new server thread safe state using defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Change the inner server state with the given one.
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

    /// Change the inner server state to running.
    pub fn set_running(&self) -> io::Result<()> {
        self.set(ServerState::Running)
    }

    /// Change the inner server state to stopping.
    pub fn set_stopping(&self) -> io::Result<()> {
        self.set(ServerState::Stopping)
    }

    /// Change the inner server state to stopped.
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

/// The server bind trait.
///
/// [`ServerBind`]ers must implement this trait.
pub trait ServerBind: Sync + Send {
    /// Describe how the server should bind to accept connections from
    /// clients.
    fn bind(&self, timer: ThreadSafeTimer) -> io::Result<()>;
}

/// The server stream trait.
///
/// [`ServerBind`]ers may implement this trait, but it is not
/// mandatory. It can be seen as a helper: by implementing the
/// [`ServerStream::read`] and the [`ServerStream::write`] functions,
/// the trait can deduce how to handle a request.
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
            Request::Set(duration) => {
                debug!("setting timer");
                timer.set(duration)?;
                Response::Ok
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

/// The server struct.
#[derive(Default)]
pub struct Server {
    /// The server configuration.
    config: ServerConfig,

    /// The current server state.
    state: ThreadSafeState,

    /// The current server timer.
    timer: ThreadSafeTimer,
}

impl Server {
    /// Start the server by running the timer in a dedicated thread as
    /// well as all the binders in dedicated threads.
    ///
    /// The main thread is then blocked by the given `wait` closure.
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

        // the tick represents the timer running in a separated thread
        let state = self.state.clone();
        let timer = self.timer.clone();
        let tick = thread::spawn(move || loop {
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
                        if let Err(err) = timer.update() {
                            warn!("cannot update timer, exiting: {err}");
                            debug!("cannot update timer: {err:?}");
                            *state = ServerState::Stopping;
                            break;
                        }
                    }
                },
                Err(err) => {
                    warn!("cannot determine if server should stop, exiting: {err}");
                    debug!("cannot determine if server should stop: {err:?}");
                    break;
                }
            }

            trace!("timer tick: {timer:#?}");
            thread::sleep(Duration::from_secs(1));
        });

        // start all binders in dedicated threads in order not to
        // block the main thread
        for binder in self.config.binders {
            let timer = self.timer.clone();
            thread::spawn(move || {
                if let Err(err) = binder.bind(timer) {
                    warn!("cannot bind, exiting: {err}");
                    debug!("cannot bind: {err:?}");
                }
            });
        }

        wait()?;

        self.state.set_stopping()?;
        fire_event(ServerEvent::Stopping);

        // wait for the timer thread to stop before exiting
        tick.join()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "cannot wait for timer thread"))?;
        fire_event(ServerEvent::Stopped);

        Ok(())
    }

    /// Wrapper around [`Server::bind_with`] where the `wait` closure
    /// sleeps every second in an infinite loop.
    pub fn bind(self) -> io::Result<()> {
        self.bind_with(|| loop {
            thread::sleep(Duration::from_secs(1));
        })
    }
}

/// The server builder.
///
/// Convenient builder to help building a final [`Server`].
#[derive(Default)]
pub struct ServerBuilder {
    /// The server configuration.
    server_config: ServerConfig,

    /// The timer configuration.
    timer_config: TimerConfig,
}

impl ServerBuilder {
    /// Create a new server builder using defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the server configuration.
    pub fn with_server_config(mut self, config: ServerConfig) -> Self {
        self.server_config = config;
        self
    }

    /// Set the timer configuration.
    pub fn with_timer_config(mut self, config: TimerConfig) -> Self {
        self.timer_config = config;
        self
    }

    /// Configure the timer to follow the Pomodoro time management
    /// method, which alternates 25 min of work and 5 min of breaks 4
    /// times, then ends with a long break of 15 min.
    ///
    /// See <https://en.wikipedia.org/wiki/Pomodoro_Technique>.
    pub fn with_pomodoro_config(mut self) -> Self {
        let work = TimerCycle::new("Work", 25 * 60);
        let short_break = TimerCycle::new("Short break", 5 * 60);
        let long_break = TimerCycle::new("Long break", 15 * 60);

        *self.timer_config.cycles = vec![
            work.clone(),
            short_break.clone(),
            work.clone(),
            short_break.clone(),
            work.clone(),
            short_break.clone(),
            work.clone(),
            short_break.clone(),
            long_break,
        ];
        self
    }

    /// Configure the timer to follow the 52/17 time management
    /// method, which alternates 52 min of work and 17 min of resting.
    ///
    /// See <https://en.wikipedia.org/wiki/52/17_rule>.
    pub fn with_52_17_config(mut self) -> Self {
        let work = TimerCycle::new("Work", 52 * 60);
        let rest = TimerCycle::new("Rest", 17 * 60);

        *self.timer_config.cycles = vec![work, rest];
        self
    }

    /// Set the server handler.
    pub fn with_server_handler<H>(mut self, handler: H) -> Self
    where
        H: Fn(ServerEvent) -> io::Result<()> + Sync + Send + 'static,
    {
        self.server_config.handler = Arc::new(handler);
        self
    }

    /// Push the given server binder.
    pub fn with_binder(mut self, binder: Box<dyn ServerBind>) -> Self {
        self.server_config.binders.push(binder);
        self
    }

    /// Set the timer handler.
    pub fn with_timer_handler<H>(mut self, handler: H) -> Self
    where
        H: Fn(TimerEvent) -> io::Result<()> + Sync + Send + 'static,
    {
        self.timer_config.handler = Arc::new(handler);
        self
    }

    /// Push the given timer cycle.
    pub fn with_cycle<C>(mut self, cycle: C) -> Self
    where
        C: Into<TimerCycle>,
    {
        self.timer_config.cycles.push(cycle.into());
        self
    }

    /// Set the timer cycles.
    pub fn with_cycles<C, I>(mut self, cycles: I) -> Self
    where
        C: Into<TimerCycle>,
        I: IntoIterator<Item = C>,
    {
        for cycle in cycles {
            self.timer_config.cycles.push(cycle.into());
        }
        self
    }

    /// Set the timer cycles count.
    pub fn with_cycles_count(mut self, count: impl Into<TimerLoop>) -> Self {
        self.timer_config.cycles_count = count.into();
        self
    }

    /// Build the final server.
    pub fn build(self) -> io::Result<Server> {
        Ok(Server {
            config: self.server_config,
            state: ThreadSafeState::new(),
            timer: ThreadSafeTimer::new(self.timer_config)?,
        })
    }
}
