use log::{error, warn};
use serde::{Deserialize, Serialize};
use std::{
    io,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

const DEFAULT_WORK_DURATION: usize = 25 * 60;
const DEFAULT_SHORT_BREAK_DURATION: usize = 5 * 60;
const DEFAULT_LONG_BREAK_DURATION: usize = 15 * 60;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Cycle {
    #[default]
    Work1,
    ShortBreak1,
    Work2,
    ShortBreak2,
    LongBreak,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum State {
    Running,
    Paused,
    #[default]
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Timer {
    pub state: State,
    pub cycle: Cycle,
    pub value: usize,
    pub work_duration: usize,
    pub short_break_duration: usize,
    pub long_break_duration: usize,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            state: State::default(),
            cycle: Cycle::default(),
            value: DEFAULT_WORK_DURATION,
            work_duration: DEFAULT_WORK_DURATION,
            short_break_duration: DEFAULT_SHORT_BREAK_DURATION,
            long_break_duration: DEFAULT_LONG_BREAK_DURATION,
        }
    }
}

impl Iterator for Timer {
    type Item = Timer;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::Running if self.value <= 1 => match self.cycle {
                Cycle::Work1 => {
                    self.cycle = Cycle::ShortBreak1;
                    self.value = self.short_break_duration;
                }
                Cycle::ShortBreak1 => {
                    self.cycle = Cycle::Work2;
                    self.value = self.work_duration;
                }
                Cycle::Work2 => {
                    self.cycle = Cycle::ShortBreak2;
                    self.value = self.short_break_duration;
                }
                Cycle::ShortBreak2 => {
                    self.cycle = Cycle::LongBreak;
                    self.value = self.long_break_duration;
                }
                Cycle::LongBreak => {
                    self.cycle = Cycle::Work1;
                    self.value = self.work_duration;
                }
            },
            State::Running => {
                self.value -= 1;
            }
            State::Paused => (),
            State::Stopped => (),
        }

        Some(self.clone())
    }
}

#[derive(Clone, Debug, Default)]
pub struct ThreadSafeTimer(Arc<Mutex<Timer>>);

impl ThreadSafeTimer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self) -> io::Result<()> {
        {
            let mut timer = self
                .0
                .lock()
                .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

            timer.state = State::Running;
            timer.cycle = Cycle::Work1;
            timer.value = timer.work_duration;
        }

        Ok(())
    }

    pub fn get(&self) -> io::Result<Timer> {
        let timer = self
            .0
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        Ok(timer.clone())
    }

    pub fn pause(&self) -> io::Result<()> {
        let mut timer = self
            .0
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        timer.state = State::Paused;

        Ok(())
    }

    pub fn resume(&self) -> io::Result<()> {
        let mut timer = self
            .0
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        timer.state = State::Running;

        Ok(())
    }

    pub fn stop(&self) -> io::Result<()> {
        let mut timer = self
            .0
            .lock()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

        timer.state = State::Stopped;
        timer.cycle = Cycle::Work1;
        timer.value = timer.work_duration;

        Ok(())
    }
}

impl Deref for ThreadSafeTimer {
    type Target = Arc<Mutex<Timer>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ThreadSafeTimer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

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
