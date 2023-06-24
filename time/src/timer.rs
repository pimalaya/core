//! # Timer module
//!
//! The [`Timer`] is composed of a [`TimerCycle`] and a
//! [`TimerState`]. The [`Timer`] is an [`Iterator`], which means it
//! knows how to switch between cycles.

use log::{error, warn};
#[cfg(test)]
use mock_instant::Instant;
use serde::{Deserialize, Serialize};
#[cfg(not(test))]
use std::time::Instant;
use std::{
    fmt, io,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum TimerCyclesCount {
    #[default]
    Infinite,
    Fixed(usize),
}

/// List of all configured [`Cycle`]s for the current [`Timer`]. It is
/// used as an inifinite loop: when the last cycle ends, the first one
/// starts back.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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

impl Eq for TimerCycle {}
impl PartialEq for TimerCycle {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.duration == other.duration
    }
}

impl TimerCycle {
    pub fn new(name: impl ToString, duration: usize) -> Self {
        Self {
            name: name.to_string(),
            duration,
            ..Self::default()
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
    pub cycles_count: TimerCyclesCount,
    pub handler: TimerChangedHandler,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            cycles: Default::default(),
            cycles_count: Default::default(),
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

    /// The current timer state.
    pub state: TimerState,

    /// The current timer cycle.
    pub cycle: TimerCycle,
    /// The current cycles counter.
    pub cycles_count: TimerCyclesCount,

    #[cfg(feature = "server")]
    #[serde(skip)]
    pub started_at: Option<Instant>,
    pub elapsed: usize,
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
        self.state == other.state && self.cycle == other.cycle && self.elapsed() == other.elapsed()
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

    pub fn update(&mut self) {
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

                if let TimerCyclesCount::Fixed(cycles_count) = self.cycles_count {
                    if elapsed > (total_duration * cycles_count) {
                        self.state = TimerState::Stopped;
                        return;
                    }
                }

                elapsed = elapsed % total_duration;

                let last_cycle = cycles[cycles.len() - 1].clone();
                let next_cycle = cycles
                    .into_iter()
                    .fold(None, |next_cycle, mut cycle| match next_cycle {
                        None if elapsed < cycle.duration => {
                            cycle.duration = cycle.duration - elapsed;
                            Some(cycle)
                        }
                        _ => next_cycle,
                    })
                    .unwrap_or(last_cycle);

                self.fire_event(TimerEvent::Running(self.cycle.clone()));

                if self.cycle.name != next_cycle.name {
                    let mut prev_cycle = self.cycle.clone();
                    prev_cycle.duration = 0;
                    self.fire_events([
                        TimerEvent::Ended(prev_cycle),
                        TimerEvent::Began(next_cycle.clone()),
                    ]);
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

    pub fn start(&mut self) -> io::Result<()> {
        if matches!(self.state, TimerState::Stopped) {
            self.state = TimerState::Running;
            self.cycle = self.config.clone_first_cycle()?;
            self.cycles_count = self.config.cycles_count.clone();
            self.started_at = Some(Instant::now());
            self.elapsed = 0;
            self.fire_events([TimerEvent::Started, TimerEvent::Began(self.cycle.clone())]);
        }
        Ok(())
    }

    pub fn set(&mut self, duration: usize) -> io::Result<()> {
        self.cycle.duration = duration;
        self.fire_event(TimerEvent::Set(self.cycle.clone()));
        Ok(())
    }

    pub fn pause(&mut self) -> io::Result<()> {
        if matches!(self.state, TimerState::Running) {
            self.state = TimerState::Paused;
            self.elapsed = self.elapsed();
            self.started_at = None;
            self.fire_event(TimerEvent::Paused(self.cycle.clone()));
        }
        Ok(())
    }

    pub fn resume(&mut self) -> io::Result<()> {
        if matches!(self.state, TimerState::Paused) {
            self.state = TimerState::Running;
            self.started_at = Some(Instant::now());
            self.fire_event(TimerEvent::Resumed(self.cycle.clone()));
        }
        Ok(())
    }

    pub fn stop(&mut self) -> io::Result<()> {
        if matches!(self.state, TimerState::Running) {
            self.state = TimerState::Stopped;
            self.fire_events([TimerEvent::Ended(self.cycle.clone()), TimerEvent::Stopped]);
            self.cycle = self.config.clone_first_cycle()?;
            self.cycles_count = self.config.cycles_count.clone();
            self.started_at = None;
            self.elapsed = 0;
        }
        Ok(())
    }

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
        timer.cycles_count = timer.config.cycles_count.clone();

        Ok(Self(Arc::new(Mutex::new(timer))))
    }

    pub fn with_timer<T>(&self, run: impl Fn(MutexGuard<Timer>) -> io::Result<T>) -> io::Result<T> {
        let timer = self
            .0
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        run(timer)
    }

    pub fn update(&self) -> io::Result<()> {
        self.with_timer(|mut timer| Ok(timer.update()))
    }

    pub fn start(&self) -> io::Result<()> {
        self.with_timer(|mut timer| timer.start())
    }

    pub fn get(&self) -> io::Result<Timer> {
        self.with_timer(|timer| Ok(timer.clone()))
    }

    pub fn set(&self, duration: usize) -> io::Result<()> {
        self.with_timer(|mut timer| timer.set(duration))
    }

    pub fn pause(&self) -> io::Result<()> {
        self.with_timer(|mut timer| timer.pause())
    }

    pub fn resume(&self) -> io::Result<()> {
        self.with_timer(|mut timer| timer.resume())
    }

    pub fn stop(&self) -> io::Result<()> {
        self.with_timer(|mut timer| timer.stop())
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
    use mock_instant::{Instant, MockClock};
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

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
            started_at: Some(Instant::now()),
            ..Default::default()
        }
    }

    #[test]
    fn running_infinite_timer() {
        let mut timer = testing_timer();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));

        // next ticks: state should still be running, cycle name
        // should be the same and cycle duration should be decremented
        // by 2

        MockClock::advance(Duration::from_secs(2));
        timer.update();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 1));

        // next tick: state should still be running, cycle should
        // switch to the next one

        MockClock::advance(Duration::from_secs(1));
        timer.update();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("b", 2));

        // next ticks: state should still be running, cycle should
        // switch to the next one

        MockClock::advance(Duration::from_secs(2));
        timer.update();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("c", 1));

        // next tick: state should still be running, cycle should
        // switch back to the first one

        MockClock::advance(Duration::from_secs(1));
        timer.update();

        assert_eq!(timer.state, TimerState::Running);
        assert_eq!(timer.cycle, TimerCycle::new("a", 3));
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
        MockClock::advance(Duration::from_secs(1));
        timer.update();
        MockClock::advance(Duration::from_secs(1));
        timer.update();
        MockClock::advance(Duration::from_secs(1));
        timer.update();
        MockClock::advance(Duration::from_secs(1));
        timer.update();

        assert_eq!(
            *events.lock().unwrap(),
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

    #[test]
    fn paused_timer_not_impacted_by_iterator() {
        let mut timer = testing_timer();
        timer.state = TimerState::Paused;
        let prev_timer = timer.clone();
        timer.update();
        assert_eq!(prev_timer, timer);
    }

    #[test]
    fn stopped_timer_not_impacted_by_iterator() {
        let mut timer = testing_timer();
        timer.state = TimerState::Stopped;
        let prev_timer = timer.clone();
        timer.update();
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
                ..Default::default()
            }
        );

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Running,
                cycle: TimerCycle::new("a", 21),
                ..Default::default()
            }
        );

        timer.pause().unwrap();

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Paused,
                cycle: TimerCycle::new("a", 21),
                ..Default::default()
            }
        );

        timer.resume().unwrap();

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Running,
                cycle: TimerCycle::new("a", 21),
                ..Default::default()
            }
        );

        timer.stop().unwrap();

        assert_eq!(
            timer.get().unwrap(),
            Timer {
                state: TimerState::Stopped,
                cycle: TimerCycle::new("a", 3),
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
