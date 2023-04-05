use super::timer::Timer;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    Ok,
    Timer(Timer),
}
