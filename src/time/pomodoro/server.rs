use log::{debug, error, trace, warn};
use std::{
    io,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use super::{timer::ThreadSafeTimer, Protocol};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum State {
    Running,
    Stopping,
    #[default]
    Stopped,
}

#[derive(Clone, Debug, Default)]
pub struct ThreadSafeState(Arc<Mutex<State>>);

impl Deref for ThreadSafeState {
    type Target = Arc<Mutex<State>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ThreadSafeState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default)]
pub struct Server {
    state: ThreadSafeState,
    timer: ThreadSafeTimer,
    protocols: Vec<Box<dyn Protocol>>,
}

impl Server {
    pub fn new(protocols: Vec<Box<dyn Protocol>>) -> Self {
        Self {
            protocols,
            ..Self::default()
        }
    }

    pub fn start(&self) -> io::Result<thread::JoinHandle<()>> {
        debug!("starting server");

        let timer = self.timer.clone();
        let state = self.state.clone();
        let tick = thread::spawn(move || {
            for timer in timer {
                match state.lock() {
                    Ok(state) => match *state {
                        State::Stopping => break,
                        State::Stopped => break,
                        State::Running => {}
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

        for protocol in &self.protocols {
            // let state = self.state.clone();
            // let listener = TcpListener::bind(&self.addr)?;
            protocol.bind(self.timer.clone())?;
        }

        Ok(tick)
    }

    pub fn stop(&self) -> io::Result<()> {
        debug!("stopping server");

        let mut state = self
            .state
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        *state = State::Stopping;

        Ok(())
    }
}
