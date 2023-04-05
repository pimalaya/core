#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    Start,
    Get,
    Pause,
    Resume,
    Stop,
}
