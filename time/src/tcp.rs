//! # TCP
//!
//! This module contains shared TCP code for both server and
//! client.

use serde::{Deserialize, Serialize};
use tokio::{
    io::{self, BufReader, ReadHalf, WriteHalf},
    net::TcpStream,
};

/// The TCP stream handler struct.
///
/// Wrapper around a TCP stream reader and writer.
pub struct TcpHandler {
    /// The TCP stream reader.
    pub reader: BufReader<ReadHalf<TcpStream>>,

    /// The TCP stream writer.
    pub writer: WriteHalf<TcpStream>,
}

impl From<TcpStream> for TcpHandler {
    fn from(stream: TcpStream) -> Self {
        let (reader, writer) = io::split(stream);
        let reader = BufReader::new(reader);
        Self { reader, writer }
    }
}

/// The TCP shared configuration between clients and servers.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TcpConfig {
    /// The TCP host name.
    pub host: String,

    /// The TCP port.
    pub port: u16,
}
