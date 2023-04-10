const DEFAULT_WORK_DURATION: usize = 25 * 60;
const DEFAULT_SHORT_BREAK_DURATION: usize = 5 * 60;
const DEFAULT_LONG_BREAK_DURATION: usize = 15 * 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub work_duration: usize,
    pub short_break_duration: usize,
    pub long_break_duration: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work_duration: DEFAULT_WORK_DURATION,
            short_break_duration: DEFAULT_SHORT_BREAK_DURATION,
            long_break_duration: DEFAULT_LONG_BREAK_DURATION,
        }
    }
}
