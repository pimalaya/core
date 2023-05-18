//! # Timer module
//!
//! The [`Timer`] is composed of a [`TimerCycle`] and a
//! [`TimerState`]. The [`Timer`] is an [`Iterator`], which means it
//! knows how to switch between cycles.

use log::{error, warn};
use serde::{Deserialize, Serialize};
use std::{
    fmt, io,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

/// List of all configured [`Cycle`]s for the current [`Timer`]. It is
/// used as an inifinite loop: when the last cycle ends, the first one
/// starts back.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimerCycles(Vec<TimerCycle>);

impl<T: IntoIterator<Item = TimerCycle>> From<T> for TimerCycles {
    fn from(value: T) -> Self {
        Self(value.into_iter().collect())
    }
}

impl Deref for TimerCycles {
    type Target = Vec<TimerCycle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TimerCycles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimerCycle {
    /// Custom name of the timer cycle.
    pub name: String,
    /// Duration of the timer cycle. This field has two meanings,
    /// depending on where it is used. *From the config point of
    /// view*, the duration represents the total duration of the
    /// cycle. *From the timer point of view*, the duration represents
    /// the amount of time remaining before the cycle ends.
    pub duration: usize,
}

impl TimerCycle {
    pub fn new<N>(name: N, duration: usize) -> Self
    where
        N: ToString,
    {
        Self {
            name: name.to_string(),
            duration,
        }
    }
}

impl<T: ToString> From<(T, usize)> for TimerCycle {
    fn from((name, duration): (T, usize)) -> Self {
        Self::new(name, duration)
    }
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
    Set(TimerCycle),
    Paused(TimerCycle),
    Resumed(TimerCycle),
    Ended(TimerCycle),
    Stopped,
}

pub type TimerChangedHandler = Arc<dyn Fn(TimerEvent) -> io::Result<()> + Sync + Send + 'static>;

#[derive(Clone)]
pub struct TimerConfig {
    pub cycles: TimerCycles,
    pub handler: TimerChangedHandler,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            cycles: TimerCycles::default(),
            handler: Arc::new(|_| Ok(())),
        }
    }
}

impl TimerConfig {
    fn clone_first_cycle(&self) -> io::Result<TimerCycle> {
        self.cycles.first().cloned().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "cannot find first cycle from timer config",
            )
        })
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Timer {
    #[serde(skip)]
    pub config: TimerConfig,
    pub state: TimerState,
    /// The active timer cycle.
    pub cycle: TimerCycle,
    /// Index in the config cycles where the active cycle inherits
    /// from.
    pub cycle_idx: usize,
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
        self.state == other.state && self.cycle == other.cycle
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
            TimerState::Running if self.cycle.duration <= 1 => {
                let next_cycle_idx = (self.cycle_idx + 1) % self.config.cycles.len();
                let next_cycle = self.config.cycles[next_cycle_idx].clone();
                self.fire_events([
                    TimerEvent::Ended(self.cycle.clone()),
                    TimerEvent::Began(next_cycle.clone()),
                ]);
                self.cycle = next_cycle;
                self.cycle_idx = next_cycle_idx;
            }
            TimerState::Running => {
                self.cycle.duration -= 1;
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
    pub fn new(config: TimerConfig) -> io::Result<Self> {
        let mut timer = Timer::default();
        timer.config = config;
        timer.cycle = timer.config.clone_first_cycle()?;
        timer.cycle_idx = 0;

        Ok(Self(Arc::new(Mutex::new(timer))))
    }

    pub fn with_timer<T>(&self, run: impl Fn(MutexGuard<Timer>) -> io::Result<T>) -> io::Result<T> {
        run(self
            .0
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?)
    }

    pub fn start(&self) -> io::Result<()> {
        self.with_timer(|mut timer| {
            timer.state = TimerState::Running;
            timer.cycle = timer.config.clone_first_cycle()?;
            timer.fire_events([TimerEvent::Started, TimerEvent::Began(timer.cycle.clone())]);
            Ok(())
        })
    }

    pub fn get(&self) -> io::Result<Timer> {
        self.with_timer(|timer| Ok(timer.clone()))
    }

    pub fn set(&self, duration: usize) -> io::Result<()> {
        self.with_timer(|mut timer| {
            timer.cycle.duration = duration;
            timer.fire_event(TimerEvent::Set(timer.cycle.clone()));
            Ok(())
        })
    }

    pub fn pause(&self) -> io::Result<()> {
        self.with_timer(|mut timer| {
            timer.state = TimerState::Paused;
            timer.fire_event(TimerEvent::Paused(timer.cycle.clone()));
            Ok(())
        })
    }

    pub fn resume(&self) -> io::Result<()> {
        self.with_timer(|mut timer| {
            timer.state = TimerState::Running;
            timer.fire_event(TimerEvent::Resumed(timer.cycle.clone()));
            Ok(())
        })
    }

    pub fn stop(&self) -> io::Result<()> {
        self.with_timer(|mut timer| {
            timer.state = TimerState::Stopped;
            timer.fire_events([TimerEvent::Ended(timer.cycle.clone()), TimerEvent::Stopped]);
            timer.cycle = timer.config.clone_first_cycle()?;
            timer.cycle_idx = 0;
            Ok(())
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{Timer, TimerConfig, TimerCycle, TimerCycles, TimerEvent, TimerState};

    fn testing_timer() -> Timer {
        Timer {
            config: TimerConfig {
                cycles: TimerCycles::from([
                    TimerCycle::new("a", 3),
                    TimerCycle::new("b", 2),
                    TimerCycle::new("c", 1),
                ]),
                ..Default::default()
            },
            state: TimerState::Running,
            cycle: TimerCycle::new("a", 3),
            cycle_idx: 0,
        }
    }

    #[test]
    fn running_timer_infinite_iterator() {
        let mut timer = testing_timer();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));
        assert_eq!(timer.cycle_idx, 0);

        // next ticks: state should still be running, cycle name
        // should be the same and cycle duration should be decremented
        // by 2

        timer.next().unwrap();
        timer.next().unwrap();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 1));
        assert_eq!(timer.cycle_idx, 0);

        // next tick: state should still be running, cycle should
        // switch to the next one

        timer.next().unwrap();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("b", 2));
        assert_eq!(timer.cycle_idx, 1);

        // next ticks: state should still be running, cycle should
        // switch to the next one

        timer.next().unwrap();
        timer.next().unwrap();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("c", 1));
        assert_eq!(timer.cycle_idx, 2);

        // next tick: state should still be running, cycle should
        // switch back to the first one

        timer.next().unwrap();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));
        assert_eq!(timer.cycle_idx, 0);
    }

    #[test]
    fn running_timer_events() {
        let mut timer = testing_timer();
        let events: Arc<Mutex<Vec<TimerEvent>>> = Arc::new(Mutex::new(Vec::new()));

        let events_for_closure = events.clone();
        timer.config.handler = Arc::new(move |evt| {
            let mut events = events_for_closure.lock().unwrap();
            events.push(evt);
            Ok(())
        });

        // from a3 to b1
        timer.next().unwrap();
        timer.next().unwrap();
        timer.next().unwrap();
        timer.next().unwrap();

        assert_eq!(
            *events.lock().unwrap(),
            vec![
                TimerEvent::Running(TimerCycle::new("a", 2)),
                TimerEvent::Running(TimerCycle::new("a", 1)),
                TimerEvent::Ended(TimerCycle::new("a", 1)),
                TimerEvent::Began(TimerCycle::new("b", 2)),
                TimerEvent::Running(TimerCycle::new("b", 1)),
            ]
        );
    }

    #[test]
    fn paused_timer_not_impacted_by_iterator() {
        let mut timer = testing_timer();
        timer.state = TimerState::Paused;
        let prev_timer = timer.clone();
        timer.next().unwrap();
        assert_eq!(prev_timer, timer);
    }

    #[test]
    fn stopped_timer_not_impacted_by_iterator() {
        let mut timer = testing_timer();
        timer.state = TimerState::Stopped;
        let prev_timer = timer.clone();
        timer.next().unwrap();
        assert_eq!(prev_timer, timer);
    }

    #[cfg(feature = "server")]
    #[test]
    fn thread_safe_timer() {
        use crate::ThreadSafeTimer;

        let mut timer = testing_timer();
        let events: Arc<Mutex<Vec<TimerEvent>>> = Arc::new(Mutex::new(Vec::new()));

        let events_for_closure = events.clone();
        timer.config.handler = Arc::new(move |evt| {
            let mut events = events_for_closure.lock().unwrap();
            events.push(evt);
            Ok(())
        });
        let timer = ThreadSafeTimer::new(timer.config).unwrap();

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Stopped,
                cycle: TimerCycle::new("a", 3),
                cycle_idx: 0,
                ..Default::default()
            }
        );

        timer.start().unwrap();
        timer.set(21).unwrap();

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Running,
                cycle: TimerCycle::new("a", 21),
                cycle_idx: 0,
                ..Default::default()
            }
        );

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Running,
                cycle: TimerCycle::new("a", 21),
                cycle_idx: 0,
                ..Default::default()
            }
        );

        timer.pause().unwrap();

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Paused,
                cycle: TimerCycle::new("a", 21),
                cycle_idx: 0,
                ..Default::default()
            }
        );

        timer.resume().unwrap();

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Running,
                cycle: TimerCycle::new("a", 21),
                cycle_idx: 0,
                ..Default::default()
            }
        );

        timer.stop().unwrap();

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Stopped,
                cycle: TimerCycle::new("a", 3),
                cycle_idx: 0,
                ..Default::default()
            }
        );

        assert_eq!(
            *events.lock().unwrap(),
            vec![
                TimerEvent::Started,
                TimerEvent::Began(TimerCycle::new("a", 3)),
                TimerEvent::Set(TimerCycle::new("a", 21)),
                TimerEvent::Paused(TimerCycle::new("a", 21)),
                TimerEvent::Resumed(TimerCycle::new("a", 21)),
                TimerEvent::Ended(TimerCycle::new("a", 21)),
                TimerEvent::Stopped,
            ]
        );
    }
}
