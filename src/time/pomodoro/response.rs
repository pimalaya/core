use super::timer::Timer;

#[derive(Debug)]
pub enum Response {
    Ok,
    Start,
    Get(Timer),
    Stop,
    Close,
}
