//! # Async-std
//!
//! This module contains the async I/O connector based on
//! [`async_std`] for [`RipStarttls`](super::RipStarttls).

use std::io::Result;

use async_std::{
    io::{BufReadExt, BufReader, WriteExt},
    net::TcpStream,
};

use super::{Event, State};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RipStarttls {
    state: super::RipStarttls,
}

impl RipStarttls {
    pub fn new(handshake_discarded: bool) -> Self {
        let state = super::RipStarttls::new(handshake_discarded);
        Self { state }
    }

    pub async fn do_starttls_prefix(mut self, mut stream: TcpStream) -> Result<TcpStream> {
        let mut event = None;

        while let Some(output) = self.state.resume(event.take()) {
            match output {
                State::DiscardHandshake => {
                    let mut line = String::new();
                    let mut reader = BufReader::new(stream);
                    reader.read_line(&mut line).await?;
                    event = Some(Event::HandshakeDiscarded(line));
                    stream = reader.into_inner();
                }
                State::WriteStarttlsCommand => {
                    let cmd = super::RipStarttls::COMMAND;
                    let count = stream.write(cmd.as_bytes()).await?;
                    event = Some(Event::StarttlsCommandWrote(count));
                }
                State::DiscardResponse => {
                    let mut line = String::new();
                    let mut reader = BufReader::new(stream);
                    reader.read_line(&mut line).await?;
                    event = Some(Event::ResponseDiscarded(line));
                    stream = reader.into_inner();
                }
            }
        }

        Ok(stream)
    }
}
