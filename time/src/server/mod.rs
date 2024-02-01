//! # Server module.
//!
//! The [`Server`] runs the timer, accepts connections from clients
//! and sends responses. The [`Server`] accepts connections using
//! [`ServerBind`]ers. The [`Server`] should have at least one
//! [`ServerBind`], otherwise it stops by itself.

#[cfg(feature = "tcp-binder")]
mod tcp;
use async_trait::async_trait;
#[cfg(feature = "tcp-binder")]
pub use tcp::*;

use log::{debug, trace};
use std::{
    future::Future,
    io,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};
use tokio::{sync::Mutex, task, time};

use crate::{RequestReader, ResponseWriter, TimerCycle, TimerLoop};

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
    async fn set(&self, next_state: ServerState) {
        let mut state = self.lock().await;
        *state = next_state;
    }

    /// Change the inner server state to running.
    pub async fn set_running(&self) {
        self.set(ServerState::Running).await
    }

    /// Change the inner server state to stopping.
    pub async fn set_stopping(&self) {
        self.set(ServerState::Stopping).await
    }

    /// Change the inner server state to stopped.
    pub async fn set_stopped(&self) {
        self.set(ServerState::Stopped).await
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
#[async_trait]
pub trait ServerBind: Send + Sync {
    /// Describe how the server should bind to accept connections from
    /// clients.
    async fn bind(&self, timer: ThreadSafeTimer) -> io::Result<()>;
}

/// The server stream trait.
///
/// [`ServerBind`]ers may implement this trait, but it is not
/// mandatory. It can be seen as a helper: by implementing the
/// [`ServerStream::read`] and the [`ServerStream::write`] functions,
/// the trait can deduce how to handle a request.
#[async_trait]
pub trait ServerStream: RequestReader + ResponseWriter {
    async fn handle(&mut self, timer: ThreadSafeTimer) -> io::Result<()> {
        let req = self.read().await?;
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
        self.write(res).await?;
        Ok(())
    }
}

impl<T: RequestReader + ResponseWriter> ServerStream for T {}

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
    pub async fn bind_with<F: Future<Output = io::Result<()>>>(
        self,
        wait: impl FnOnce() -> F,
    ) -> io::Result<()> {
        debug!("starting server");

        let fire_event = |event: ServerEvent| {
            if let Err(err) = (self.config.handler)(event.clone()) {
                debug!("cannot fire event {event:?}: {err}");
                debug!("{err:?}");
            }
        };

        self.state.set_running().await;
        fire_event(ServerEvent::Started);

        // the tick represents the timer running in a separated thread
        let state = self.state.clone();
        let timer = self.timer.clone();
        let tick = task::spawn(async move {
            loop {
                let mut state = state.lock().await;
                match *state {
                    ServerState::Stopping => {
                        *state = ServerState::Stopped;
                        break;
                    }
                    ServerState::Stopped => {
                        break;
                    }
                    ServerState::Running => {
                        if let Err(err) = timer.update() {
                            debug!("cannot update timer, exiting: {err}");
                            debug!("{err:?}");
                            *state = ServerState::Stopping;
                            break;
                        }
                    }
                };
                drop(state);

                trace!("timer tick: {timer:#?}");
                time::sleep(Duration::from_secs(1)).await;
            }
        });

        // start all binders in dedicated threads in order not to
        // block the main thread
        for binder in self.config.binders {
            let timer = self.timer.clone();
            task::spawn(async move {
                if let Err(err) = binder.bind(timer).await {
                    debug!("cannot bind, exiting: {err}");
                    debug!("{err:?}");
                }
            });
        }

        wait().await?;

        self.state.set_stopping().await;
        fire_event(ServerEvent::Stopping);

        // wait for the timer thread to stop before exiting
        tick.await
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "cannot wait for timer thread"))?;
        fire_event(ServerEvent::Stopped);

        Ok(())
    }

    /// Wrapper around [`Server::bind_with`] where the `wait` closure
    /// sleeps every second in an infinite loop.
    pub async fn bind(self) -> io::Result<()> {
        self.bind_with(|| async {
            loop {
                time::sleep(Duration::from_secs(1)).await;
            }
        })
        .await
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
