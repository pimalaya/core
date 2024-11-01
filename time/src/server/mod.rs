//! # Server
//!
//! This module contains everything related to servers. The server
//! runs the timer, accepts connections from clients and sends
//! responses. It accepts connections using server binders. A server
//! should have at least one binder, otherwise it stops by itself.
//!
//!

#[cfg(feature = "tcp-binder")]
pub mod tcp;

use std::{
    fmt::Debug,
    future::Future,
    io::Result,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "async-std")]
use async_std::task::sleep;
use async_trait::async_trait;
use futures::{lock::Mutex, select, stream::FuturesUnordered, FutureExt, StreamExt};
#[cfg(feature = "tokio")]
use tokio::time::sleep;
use tracing::{debug, trace};

use crate::{
    handler::{self, Handler},
    request::{Request, RequestReader},
    response::{Response, ResponseWriter},
    timer::{ThreadSafeTimer, TimerConfig, TimerCycle, TimerEvent, TimerLoop},
};

/// The server state enum.
///
/// Enumeration of all the possible states of a server.
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
    /// The server state changed handler.
    handler: Arc<Handler<ServerEvent>>,

    /// The binders list the server should use when starting up.
    binders: Vec<Box<dyn ServerBind>>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            handler: handler::default(),
            binders: Vec::new(),
        }
    }
}

/// The server state changed event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServerEvent {
    /// The server just started.
    Started,

    /// The server is stopping.
    Stopping,

    /// The server has stopped.
    Stopped,
}

/// Thread safe version of the server state.
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
/// Server binders must implement this trait.
#[async_trait]
pub trait ServerBind: Debug + Send + Sync {
    /// Describe how the server should bind to accept connections from
    /// clients.
    async fn bind(&self, timer: ThreadSafeTimer) -> Result<()>;
}

/// The server stream trait.
///
/// Describes how a request should be parsed and handled.
#[async_trait]
pub trait ServerStream: RequestReader + ResponseWriter {
    /// Read the request, process it then write the response.
    async fn handle(&mut self, timer: ThreadSafeTimer) -> Result<()> {
        let req = self.read().await?;
        let res = match req {
            Request::Start => {
                debug!("starting timer");
                timer.start().await?;
                Response::Ok
            }
            Request::Get => {
                debug!("getting timer");
                let timer = timer.get().await;
                trace!("{timer:#?}");
                Response::Timer(timer)
            }
            Request::Set(duration) => {
                debug!("setting timer");
                timer.set(duration).await?;
                Response::Ok
            }
            Request::Pause => {
                debug!("pausing timer");
                timer.pause().await?;
                Response::Ok
            }
            Request::Resume => {
                debug!("resuming timer");
                timer.resume().await?;
                Response::Ok
            }
            Request::Stop => {
                debug!("stopping timer");
                timer.stop().await?;
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
    pub async fn bind_with<F: Future<Output = Result<()>> + Send + 'static>(
        self,
        wait: impl FnOnce() -> F + Send + Sync + 'static,
    ) -> Result<()> {
        debug!("starting server");

        let handler = &self.config.handler;
        let fire_event = |event: ServerEvent| async move {
            debug!("firing server event {event:?}");

            if let Err(err) = handler(event.clone()).await {
                debug!("error while firing server event, skipping it");
                debug!("{err:?}");
            }
        };

        self.state.set_running().await;
        fire_event(ServerEvent::Started).await;

        // the tick represents the timer running in a separated thread
        let state = self.state.clone();
        let timer = self.timer.clone();
        let tick = spawn(async move {
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
                        timer.update().await;
                    }
                };
                drop(state);

                sleep(Duration::from_secs(1)).await;
            }
        });

        // start all binders in dedicated threads in order not to
        // block the main thread

        let binders = FuturesUnordered::from_iter(self.config.binders.into_iter().map(|binder| {
            let timer = self.timer.clone();
            spawn(async move {
                debug!("binding {binder:?}");
                if let Err(err) = binder.bind(timer).await {
                    debug!("error while binding, skipping it");
                    debug!("{err:?}");
                }
            })
        }))
        .filter_map(|res| async {
            match res {
                Ok(res) => Some(res),
                Err(err) => {
                    debug!(?err, "skipping failed task");
                    None
                }
            }
        })
        .collect::<()>();

        debug!("main loop started");
        select! {
            _ = tick.fuse() => (),
            _ = binders.fuse() => (),
            _ = wait().fuse() => (),
        };
        debug!("main loop stopped");

        self.state.set_stopping().await;
        fire_event(ServerEvent::Stopping).await;

        // wait for the timer thread to stop before exiting
        // tick.await
        //     .map_err(|_| Error::new(ErrorKind::Other, "cannot wait for timer thread"))?;
        fire_event(ServerEvent::Stopped).await;

        Ok(())
    }

    /// Wrapper around [`Server::bind_with`] where the `wait` closure
    /// sleeps every second in an infinite loop.
    pub async fn bind(self) -> Result<()> {
        self.bind_with(|| async {
            loop {
                sleep(Duration::from_secs(1)).await;
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
    pub fn with_server_handler<F: Future<Output = Result<()>> + Send + 'static>(
        mut self,
        handler: impl Fn(ServerEvent) -> F + Send + Sync + 'static,
    ) -> Self {
        self.server_config.handler = Arc::new(move |evt| Box::pin(handler(evt)));
        self
    }

    /// Push the given server binder.
    pub fn with_binder(mut self, binder: Box<dyn ServerBind>) -> Self {
        self.server_config.binders.push(binder);
        self
    }

    /// Set the timer handler.
    pub fn with_timer_handler<F: Future<Output = Result<()>> + Send + 'static>(
        mut self,
        handler: impl Fn(TimerEvent) -> F + Sync + Send + 'static,
    ) -> Self {
        self.timer_config.handler = Arc::new(move |evt| Box::pin(handler(evt)));
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
    pub fn build(self) -> Result<Server> {
        Ok(Server {
            config: self.server_config,
            state: ThreadSafeState::new(),
            timer: ThreadSafeTimer::new(self.timer_config)?,
        })
    }
}

#[cfg(feature = "async-std")]
pub(crate) async fn spawn<F>(f: F) -> Result<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    Ok(async_std::task::spawn(f).await)
}

#[cfg(feature = "tokio")]
pub(crate) async fn spawn<F>(f: F) -> Result<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    Ok(tokio::task::spawn(f).await?)
}
