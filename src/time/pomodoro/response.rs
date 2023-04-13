use super::Timer;

#[derive(Clone, Debug)]
pub enum Response {
    Ok,
    Timer(Timer),
}
