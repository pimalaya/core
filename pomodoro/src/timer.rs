//! # Timer module.
//!
//! The [`Timer`] is composed of a [`TimerCycle`] (work, short break
//! or long break), a [`TimerState`] (running, paused or stopped) and
//! a value (current seconds). The [`Timer`] is also an [`Iterator`],
//! which means it knows how to increment the value and how to change
//! between cycles.

use log::{error, warn};
use serde::{Deserialize, Serialize};
use std::{
    fmt, io,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

pub const DEFAULT_WORK_DURATION: usize = 25 * 60;
pub const DEFAULT_SHORT_BREAK_DURATION: usize = 5 * 60;
pub const DEFAULT_LONG_BREAK_DURATION: usize = 15 * 60;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum TimerCycle {
    #[default]
    FirstWork,
    FirstShortBreak,
    SecondWork,
    SecondShortBreak,
    LongBreak,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum TimerState {
    Running,
    Paused,
    #[default]
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimerEvent {
    Started,
    Began(TimerCycle),
    Running(TimerCycle),
    Paused(TimerCycle),
    Resumed(TimerCycle),
    Ended(TimerCycle),
    Stopped,
}

pub type TimerChangedHandler = Arc<dyn Fn(TimerEvent) -> io::Result<()> + Sync + Send + 'static>;

#[derive(Clone)]
pub struct TimerConfig {
    pub work_duration: usize,
    pub short_break_duration: usize,
    pub long_break_duration: usize,
    pub handler: TimerChangedHandler,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            work_duration: DEFAULT_WORK_DURATION,
            short_break_duration: DEFAULT_SHORT_BREAK_DURATION,
            long_break_duration: DEFAULT_LONG_BREAK_DURATION,
            handler: Arc::new(|_| Ok(())),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Timer {
    #[serde(skip)]
    pub config: TimerConfig,
    pub state: TimerState,
    pub cycle: TimerCycle,
    pub value: usize,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            config: TimerConfig::default(),
            state: TimerState::default(),
            cycle: TimerCycle::default(),
            value: DEFAULT_WORK_DURATION,
        }
    }
}

impl fmt::Debug for Timer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let timer = serde_json::to_string(self).map_err(|_| fmt::Error)?;
        write!(f, "{timer}")
    }
}

impl Eq for Timer {}
impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state && self.cycle == other.cycle && self.value == other.value
    }
}

#[cfg(feature = "server")]
impl Timer {
    pub fn fire_event(&self, event: TimerEvent) {
        if let Err(err) = (self.config.handler)(event.clone()) {
            warn!("cannot fire event {event:?}, skipping it");
            error!("{err}");
        }
    }

    pub fn fire_events<E: IntoIterator<Item = TimerEvent>>(&self, events: E) {
        for event in events.into_iter() {
            self.fire_event(event)
        }
    }
}

#[cfg(feature = "server")]
impl Iterator for Timer {
    type Item = Timer;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            TimerState::Running if self.value <= 1 => match self.cycle {
                TimerCycle::FirstWork => {
                    self.cycle = TimerCycle::FirstShortBreak;
                    self.value = self.config.short_break_duration;
                    self.fire_events([
                        TimerEvent::Ended(TimerCycle::FirstWork),
                        TimerEvent::Began(TimerCycle::FirstShortBreak),
                    ]);
                }
                TimerCycle::FirstShortBreak => {
                    self.cycle = TimerCycle::SecondWork;
                    self.value = self.config.work_duration;
                    self.fire_events([
                        TimerEvent::Ended(TimerCycle::FirstShortBreak),
                        TimerEvent::Began(TimerCycle::SecondWork),
                    ]);
                }
                TimerCycle::SecondWork => {
                    self.cycle = TimerCycle::SecondShortBreak;
                    self.value = self.config.short_break_duration;
                    self.fire_events([
                        TimerEvent::Ended(TimerCycle::SecondWork),
                        TimerEvent::Began(TimerCycle::SecondShortBreak),
                    ]);
                }
                TimerCycle::SecondShortBreak => {
                    self.cycle = TimerCycle::LongBreak;
                    self.value = self.config.long_break_duration;
                    self.fire_events([
                        TimerEvent::Ended(TimerCycle::SecondShortBreak),
                        TimerEvent::Began(TimerCycle::LongBreak),
                    ]);
                }
                TimerCycle::LongBreak => {
                    self.cycle = TimerCycle::FirstWork;
                    self.value = self.config.work_duration;
                    self.fire_events([
                        TimerEvent::Ended(TimerCycle::LongBreak),
                        TimerEvent::Began(TimerCycle::FirstWork),
                    ])
                }
            },
            TimerState::Running => {
                self.value -= 1;
                self.fire_event(TimerEvent::Running(self.cycle.clone()));
            }
            TimerState::Paused => {
                // nothing to do
            }
            TimerState::Stopped => {
                // nothing to do
            }
        }

        Some(self.clone())
    }
}

/// Thread safe version of the [`Timer`]. The server does not
/// manipulate directly the [`Timer`], it uses this thread safe
/// version instead (mainly because the timer runs in a
/// [`std::thread::spawn`] loop).
#[cfg(feature = "server")]
#[derive(Clone, Debug, Default)]
pub struct ThreadSafeTimer(Arc<Mutex<Timer>>);

#[cfg(feature = "server")]
impl ThreadSafeTimer {
    pub fn new(config: TimerConfig) -> Self {
        let mut timer = Timer::default();
        timer.config = config;
        Self(Arc::new(Mutex::new(timer)))
    }

    pub fn with_timer<T>(&self, run: impl Fn(MutexGuard<Timer>) -> T) -> io::Result<T> {
        Ok(run(self.0.lock().map_err(|err| {
            io::Error::new(io::ErrorKind::Other, err.to_string())
        })?))
    }

    pub fn start(&self) -> io::Result<()> {
        self.with_timer(|mut timer| {
            timer.state = TimerState::Running;
            timer.cycle = TimerCycle::FirstWork;
            timer.value = timer.config.work_duration;
            timer.fire_events([
                TimerEvent::Started,
                TimerEvent::Began(TimerCycle::FirstWork),
            ]);
        })
    }

    pub fn get(&self) -> io::Result<Timer> {
        self.with_timer(|timer| timer.clone())
    }

    pub fn pause(&self) -> io::Result<()> {
        self.with_timer(|mut timer| {
            timer.state = TimerState::Paused;
            timer.fire_event(TimerEvent::Paused(timer.cycle.clone()));
        })
    }

    pub fn resume(&self) -> io::Result<()> {
        self.with_timer(|mut timer| {
            timer.state = TimerState::Running;
            timer.fire_event(TimerEvent::Resumed(timer.cycle.clone()));
        })
    }

    pub fn stop(&self) -> io::Result<()> {
        self.with_timer(|mut timer| {
            timer.state = TimerState::Stopped;
            timer.cycle = TimerCycle::FirstWork;
            timer.value = timer.config.work_duration;
            timer.fire_events([TimerEvent::Ended(timer.cycle.clone()), TimerEvent::Stopped]);
        })
    }
}

#[cfg(feature = "server")]
impl Deref for ThreadSafeTimer {
    type Target = Arc<Mutex<Timer>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "server")]
impl DerefMut for ThreadSafeTimer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "server")]
impl Iterator for ThreadSafeTimer {
    type Item = ThreadSafeTimer;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.lock() {
            Ok(mut timer) => timer.next().map(|_| self.clone()),
            Err(err) => {
                warn!("cannot lock timer, exiting the loop");
                error!("{}", err);
                None
            }
        }
    }
}
