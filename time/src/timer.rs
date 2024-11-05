//! # Timer
//!
//! This module contains everything related to the timer. A timer can
//! be identified by a state (running or stopped), a cycle and a
//! cycles count (infinite or finite). During the lifetime of the
//! timer, timer events are triggered.

#[cfg(feature = "server")]
use std::io::{Error, ErrorKind};

#[cfg(feature = "server")]
use futures::lock::Mutex;
#[cfg(all(feature = "server", test))]
use mock_instant::Instant;
#[cfg(all(feature = "server", not(test)))]
use std::time::Instant;
use std::{
    fmt,
    io::Result,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tracing::debug;

use crate::handler::{self, Handler};

/// The timer loop.
///
/// When the timer reaches its last cycle, it starts again from the
/// first cycle. This structure defines the number of loops the timer
/// should do before stopping by itself.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum TimerLoop {
    /// The timer loops indefinitely and therefore never stops by
    /// itself.
    ///
    /// The only way to stop such timer is via a stop request.
    #[default]
    Infinite,

    /// The timer stops by itself after the given number of loops.
    Fixed(usize),
}

impl From<usize> for TimerLoop {
    fn from(count: usize) -> Self {
        if count == 0 {
            Self::Infinite
        } else {
            Self::Fixed(count)
        }
    }
}

/// The timer cycle.
///
/// A cycle is a step in the timer lifetime, represented by a name and
/// a duration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct TimerCycle {
    /// The name of the timer cycle.
    pub name: String,

    /// The duration of the timer cycle.
    ///
    /// This field has two meanings, depending on where it is
    /// used. *From the config point of view*, the duration represents
    /// the total duration of the cycle. *From the timer point of
    /// view*, the duration represents the amount of time remaining
    /// before the cycle ends.
    pub duration: usize,
}

impl TimerCycle {
    pub fn new(name: impl ToString, duration: usize) -> Self {
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

/// The timer cycles list.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct TimerCycles(Vec<TimerCycle>);

impl<T: IntoIterator<Item = TimerCycle>> From<T> for TimerCycles {
    fn from(cycles: T) -> Self {
        Self(cycles.into_iter().collect())
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

/// The timer state.
///
/// Enumeration of all the possible state of a timer: running, paused
/// or stopped.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum TimerState {
    /// The timer is running.
    Running,

    /// The timer has been paused.
    Paused,

    /// The timer is not running.
    #[default]
    Stopped,
}

/// The timer event.
///
/// Enumeration of all possible events that can be triggered during
/// the timer lifecycle.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum TimerEvent {
    /// The timer started.
    Started,

    /// The timer began the given cycle.
    Began(TimerCycle),

    /// The timer is running the given cycle (tick).
    Running(TimerCycle),

    /// The timer has been set to the given cycle.
    Set(TimerCycle),

    /// The timer has been paused at the given cycle.
    Paused(TimerCycle),

    /// The timer has been resumed at the given cycle.
    Resumed(TimerCycle),

    /// The timer ended with the given cycle.
    Ended(TimerCycle),

    /// The timer stopped.
    Stopped,
}

/// The timer configuration.
#[derive(Clone)]
pub struct TimerConfig {
    /// The list of custom timer cycles.
    pub cycles: TimerCycles,

    /// The timer cycles counter.
    pub cycles_count: TimerLoop,

    /// The timer event handler.
    pub handler: Arc<Handler<TimerEvent>>,
}

impl fmt::Debug for TimerConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TimerConfig")
            .field("cycles", &self.cycles)
            .field("cycles_count", &self.cycles_count)
            .finish()
    }
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            cycles: Default::default(),
            cycles_count: Default::default(),
            handler: handler::default(),
        }
    }
}

#[cfg(feature = "server")]
impl TimerConfig {
    fn clone_first_cycle(&self) -> Result<TimerCycle> {
        self.cycles.first().cloned().ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                "cannot find first cycle from timer config",
            )
        })
    }
}

/// The main timer struct.
#[derive(Clone, Debug, Default)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct Timer {
    /// The current timer configuration.
    #[cfg_attr(feature = "derive", serde(skip))]
    pub config: TimerConfig,

    /// The current timer state.
    pub state: TimerState,

    /// The current timer cycle.
    pub cycle: TimerCycle,

    /// The current cycles counter.
    pub cycles_count: TimerLoop,

    #[cfg(feature = "server")]
    #[cfg_attr(feature = "derive", serde(skip))]
    pub started_at: Option<Instant>,

    #[cfg(feature = "server")]
    pub elapsed: usize,
}

impl Eq for Timer {}

#[cfg(feature = "server")]
impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state && self.cycle == other.cycle && self.elapsed() == other.elapsed()
    }
}

#[cfg(not(feature = "server"))]
impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state && self.cycle == other.cycle
    }
}

#[cfg(feature = "server")]
impl Timer {
    pub fn elapsed(&self) -> usize {
        self.started_at
            .map(|i| i.elapsed().as_secs() as usize)
            .unwrap_or_default()
            + self.elapsed
    }

    pub async fn update(&mut self) {
        let mut elapsed = self.elapsed();

        match self.state {
            TimerState::Running => {
                let (cycles, total_duration) = self.config.cycles.iter().cloned().fold(
                    (Vec::new(), 0),
                    |(mut cycles, mut sum), mut cycle| {
                        cycle.duration += sum;
                        sum = cycle.duration;
                        cycles.push(cycle);
                        (cycles, sum)
                    },
                );

                if let TimerLoop::Fixed(cycles_count) = self.cycles_count {
                    if elapsed >= (total_duration * cycles_count) {
                        self.state = TimerState::Stopped;
                        return;
                    }
                }

                elapsed %= total_duration;

                let last_cycle = cycles[cycles.len() - 1].clone();
                let next_cycle = cycles
                    .into_iter()
                    .fold(None, |next_cycle, mut cycle| match next_cycle {
                        None if elapsed < cycle.duration => {
                            cycle.duration -= elapsed;
                            Some(cycle)
                        }
                        _ => next_cycle,
                    })
                    .unwrap_or(last_cycle);

                self.fire_event(TimerEvent::Running(self.cycle.clone()))
                    .await;

                if self.cycle.name != next_cycle.name {
                    let mut prev_cycle = self.cycle.clone();
                    prev_cycle.duration = 0;
                    self.fire_events([
                        TimerEvent::Ended(prev_cycle),
                        TimerEvent::Began(next_cycle.clone()),
                    ])
                    .await;
                }

                self.cycle = next_cycle;
            }
            TimerState::Paused => {
                // nothing to do
            }
            TimerState::Stopped => {
                // nothing to do
            }
        }
    }

    pub async fn fire_event(&self, event: TimerEvent) {
        let handler = &self.config.handler;
        debug!("firing timer event {event:?}");
        if let Err(err) = handler(event.clone()).await {
            debug!("cannot fire timer event, skipping it");
            debug!("{err:?}");
        }
    }

    pub async fn fire_events(&self, events: impl IntoIterator<Item = TimerEvent>) {
        for event in events.into_iter() {
            self.fire_event(event).await
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        if matches!(self.state, TimerState::Stopped) {
            self.state = TimerState::Running;
            self.cycle = self.config.clone_first_cycle()?;
            self.cycles_count = self.config.cycles_count.clone();
            self.started_at = Some(Instant::now());
            self.elapsed = 0;
            self.fire_events([TimerEvent::Started, TimerEvent::Began(self.cycle.clone())])
                .await;
        }
        Ok(())
    }

    pub async fn set(&mut self, duration: usize) -> Result<()> {
        self.cycle.duration = duration;
        self.fire_event(TimerEvent::Set(self.cycle.clone())).await;
        Ok(())
    }

    pub async fn pause(&mut self) -> Result<()> {
        if matches!(self.state, TimerState::Running) {
            self.state = TimerState::Paused;
            self.elapsed = self.elapsed();
            self.started_at = None;
            self.fire_event(TimerEvent::Paused(self.cycle.clone()))
                .await;
        }
        Ok(())
    }

    pub async fn resume(&mut self) -> Result<()> {
        if matches!(self.state, TimerState::Paused) {
            self.state = TimerState::Running;
            self.started_at = Some(Instant::now());
            self.fire_event(TimerEvent::Resumed(self.cycle.clone()))
                .await;
        }
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if matches!(self.state, TimerState::Running) {
            self.state = TimerState::Stopped;
            self.fire_events([TimerEvent::Ended(self.cycle.clone()), TimerEvent::Stopped])
                .await;
            self.cycle = self.config.clone_first_cycle()?;
            self.cycles_count = self.config.cycles_count.clone();
            self.started_at = None;
            self.elapsed = 0;
        }
        Ok(())
    }
}

/// Thread safe version of the [`Timer`].
///
/// The server does not manipulate directly the [`Timer`], it uses
/// this thread safe version instead (mainly because the timer runs in
/// a [`std::thread::spawn`] loop).
#[cfg(feature = "server")]
#[derive(Clone, Debug, Default)]
pub struct ThreadSafeTimer(Arc<Mutex<Timer>>);

#[cfg(feature = "server")]
impl ThreadSafeTimer {
    pub fn new(config: TimerConfig) -> Result<Self> {
        let mut timer = Timer::default();

        timer.config = config;
        timer.cycle = timer.config.clone_first_cycle()?;
        timer.cycles_count = timer.config.cycles_count.clone();

        Ok(Self(Arc::new(Mutex::new(timer))))
    }

    pub async fn update(&self) {
        self.0.lock().await.update().await;
    }

    pub async fn start(&self) -> Result<()> {
        self.0.lock().await.start().await
    }

    pub async fn get(&self) -> Timer {
        self.0.lock().await.clone()
    }

    pub async fn set(&self, duration: usize) -> Result<()> {
        self.0.lock().await.set(duration).await
    }

    pub async fn pause(&self) -> Result<()> {
        self.0.lock().await.pause().await
    }

    pub async fn resume(&self) -> Result<()> {
        self.0.lock().await.resume().await
    }

    pub async fn stop(&self) -> Result<()> {
        self.0.lock().await.stop().await
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

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    #[cfg(feature = "async-std")]
    use async_std::test;
    use mock_instant::{Instant, MockClock};
    use once_cell::sync::Lazy;
    #[cfg(feature = "tokio")]
    use tokio::test;

    use super::*;

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
            started_at: Some(Instant::now()),
            ..Default::default()
        }
    }

    #[test_log::test(test)]
    async fn running_infinite_timer() {
        let mut timer = testing_timer();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));

        // next ticks: state should still be running, cycle name
        // should be the same and cycle duration should be decremented
        // by 2

        MockClock::advance(Duration::from_secs(2));
        timer.update().await;

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 1));

        // next tick: state should still be running, cycle should
        // switch to the next one

        MockClock::advance(Duration::from_secs(1));
        timer.update().await;

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("b", 2));

        // next ticks: state should still be running, cycle should
        // switch to the next one

        MockClock::advance(Duration::from_secs(2));
        timer.update().await;

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("c", 1));

        // next tick: state should still be running, cycle should
        // switch back to the first one

        MockClock::advance(Duration::from_secs(1));
        timer.update().await;

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));
    }

    #[test_log::test(test)]
    async fn running_timer_events() {
        static EVENTS: Lazy<Mutex<Vec<TimerEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));

        let mut timer = testing_timer();

        timer.config.handler = Arc::new(|evt| {
            Box::pin(async {
                EVENTS.lock().await.push(evt);
                Ok(())
            })
        });

        // from a3 to b1
        MockClock::advance(Duration::from_secs(1));
        timer.update().await;
        MockClock::advance(Duration::from_secs(1));
        timer.update().await;
        MockClock::advance(Duration::from_secs(1));
        timer.update().await;
        MockClock::advance(Duration::from_secs(1));
        timer.update().await;

        assert_eq!(
            *EVENTS.lock().await,
            vec![
                TimerEvent::Running(TimerCycle::new("a", 3)),
                TimerEvent::Running(TimerCycle::new("a", 2)),
                TimerEvent::Running(TimerCycle::new("a", 1)),
                TimerEvent::Ended(TimerCycle::new("a", 0)),
                TimerEvent::Began(TimerCycle::new("b", 2)),
                TimerEvent::Running(TimerCycle::new("b", 2)),
            ]
        );
    }

    #[test_log::test(test)]
    async fn paused_timer_not_impacted_by_iterator() {
        let mut timer = testing_timer();
        timer.state = TimerState::Paused;
        let prev_timer = timer.clone();
        timer.update().await;
        assert_eq!(prev_timer, timer);
    }

    #[test_log::test(test)]
    async fn stopped_timer_not_impacted_by_iterator() {
        let mut timer = testing_timer();
        timer.state = TimerState::Stopped;
        let prev_timer = timer.clone();
        timer.update().await;
        assert_eq!(prev_timer, timer);
    }

    #[cfg(feature = "server")]
    #[test_log::test(test)]
    async fn thread_safe_timer() {
        let mut timer = testing_timer();
        static EVENTS: Lazy<Mutex<Vec<TimerEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));

        timer.config.handler = Arc::new(move |evt| {
            Box::pin(async {
                EVENTS.lock().await.push(evt);
                Ok(())
            })
        });
        let timer = ThreadSafeTimer::new(timer.config).unwrap();

        assert_eq!(
            timer.get().await,
            Timer {
                state: TimerState::Stopped,
                cycle: TimerCycle::new("a", 3),
                ..Default::default()
            }
        );

        timer.start().await.unwrap();
        timer.set(21).await.unwrap();

        assert_eq!(
            timer.get().await,
            Timer {
                state: TimerState::Running,
                cycle: TimerCycle::new("a", 21),
                ..Default::default()
            }
        );

        assert_eq!(
            timer.get().await,
            Timer {
                state: TimerState::Running,
                cycle: TimerCycle::new("a", 21),
                ..Default::default()
            }
        );

        timer.pause().await.unwrap();

        assert_eq!(
            timer.get().await,
            Timer {
                state: TimerState::Paused,
                cycle: TimerCycle::new("a", 21),
                ..Default::default()
            }
        );

        timer.resume().await.unwrap();

        assert_eq!(
            timer.get().await,
            Timer {
                state: TimerState::Running,
                cycle: TimerCycle::new("a", 21),
                ..Default::default()
            }
        );

        timer.stop().await.unwrap();

        assert_eq!(
            timer.get().await,
            Timer {
                state: TimerState::Stopped,
                cycle: TimerCycle::new("a", 3),
                ..Default::default()
            }
        );

        assert_eq!(
            *EVENTS.lock().await,
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
