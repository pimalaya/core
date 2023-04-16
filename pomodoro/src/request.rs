//! # Request module.
//!
//! A [`Request`] is the type of data sent by the client to the server
//! in order to control the timer.

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    /// Request the timer to start with the first work cycle.
    Start,
    /// Request the state, the cycle and the value of the timer.
    Get,
    /// Request to pause the timer. A paused timer freezes, which
    /// means it keeps its state, cycle and value till it get resumed.
    Pause,
    /// Request to resume the paused timer.
    Resume,
    /// Request to stop the timer. Stopping the timer resets the
    /// state, cycle and the value.
    Stop,
}
