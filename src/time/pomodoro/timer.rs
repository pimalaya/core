#[derive(Clone, Debug)]
pub enum TimerKind {
    Work1,
    ShortBreak1,
    Work2,
    ShortBreak2,
    LongBreak,
}

#[derive(Clone, Debug)]
pub struct Timer {
    kind: TimerKind,
    value: usize,
    work_duration: usize,
    short_break_duration: usize,
    long_break_duration: usize,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            kind: TimerKind::Work1,
            value: 25 * 60,
            work_duration: 25 * 60,
            short_break_duration: 5 * 60,
            long_break_duration: 15 * 60,
        }
    }
}

impl Iterator for Timer {
    type Item = Timer;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value == 1 {
            match self.kind {
                TimerKind::Work1 => {
                    self.kind = TimerKind::ShortBreak1;
                    self.value = self.short_break_duration;
                }
                TimerKind::ShortBreak1 => {
                    self.kind = TimerKind::Work2;
                    self.value = self.work_duration;
                }
                TimerKind::Work2 => {
                    self.kind = TimerKind::ShortBreak2;
                    self.value = self.short_break_duration;
                }
                TimerKind::ShortBreak2 => {
                    self.kind = TimerKind::LongBreak;
                    self.value = self.long_break_duration;
                }
                TimerKind::LongBreak => {
                    self.kind = TimerKind::Work1;
                    self.value = self.work_duration;
                }
            }
        } else {
            self.value -= 1;
        }

        Some(self.clone())
    }
}
