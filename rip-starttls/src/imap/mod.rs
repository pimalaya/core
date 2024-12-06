//! # IMAP
//!
//! This module contains the sans I/O implementation for the IMAP
//! protocol, as well as feature-gated I/O connectors.

#[cfg(feature = "async-std")]
pub mod async_std;
#[cfg(feature = "std")]
pub mod std;
#[cfg(feature = "tokio")]
pub mod tokio;

use tracing::debug;

/// The main structure of the IMAP module.
///
/// This structure allows you to move a TCP stream to a TLS-ready
/// state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RipStarttls {
    state: Option<State>,
    event: Option<Event>,
    handshake_discarded: bool,
}

impl RipStarttls {
    pub const COMMAND: &str = "A STARTTLS\r\n";

    pub fn new(handshake_discarded: bool) -> Self {
        Self {
            state: None,
            event: None,
            handshake_discarded,
        }
    }

    /// Acts like a coroutine's resume function, where the argument is
    /// replaced by an event.
    pub fn resume(&mut self, event: Option<Event>) -> Option<State> {
        self.event = event;
        self.next()
    }
}

impl Iterator for RipStarttls {
    type Item = State;

    fn next(&mut self) -> Option<State> {
        let event = self.event.take();

        match self.state {
            None => {
                self.state = Some(if self.handshake_discarded {
                    State::WriteStarttlsCommand
                } else {
                    State::DiscardHandshake
                })
            }
            Some(State::DiscardHandshake) => {
                if let Some(Event::HandshakeDiscarded(line)) = event {
                    debug!("discarded IMAP greeting: {line:?}");
                    self.state = Some(State::WriteStarttlsCommand);
                }
            }
            Some(State::WriteStarttlsCommand) => {
                if let Some(Event::StarttlsCommandWrote(_)) = event {
                    let cmd = Self::COMMAND;
                    debug!("wrote IMAP STARTTLS command: {cmd:?}");
                    self.state = Some(State::DiscardResponse);
                }
            }
            Some(State::DiscardResponse) => {
                if let Some(Event::ResponseDiscarded(line)) = event {
                    debug!("discarded IMAP response: {line:?}");

                    if line.starts_with("A ") {
                        debug!("stream ready for TLS negociation");
                        self.state = None;
                    }
                }
            }
        }

        self.state.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum State {
    DiscardHandshake,
    WriteStarttlsCommand,
    DiscardResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    HandshakeDiscarded(String),
    StarttlsCommandWrote(usize),
    ResponseDiscarded(String),
}
