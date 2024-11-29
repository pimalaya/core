#![cfg(feature = "blocking")]

use std::{
    env,
    net::{Shutdown, TcpStream},
};

use start_tls::{blocking::PrepareStartTls, imap::ImapStartTls};

const READ_BUF_CAPACITY: usize = 1024;

fn main() {
    env_logger::builder().is_test(true).init();

    let host = env::var("HOST").expect("HOST should be defined");
    let port: u16 = env::var("PORT")
        .expect("PORT should be defined")
        .parse()
        .expect("PORT should be an unsigned integer");

    println!("connecting to {host}:{port} using TCP…");
    let mut tcp_stream =
        TcpStream::connect((host.as_str(), port)).expect("should connect to TCP stream");

    println!("preparing TCP connection for STARTTLS…");
    ImapStartTls::new()
        .with_read_buffer_capacity(READ_BUF_CAPACITY)
        .with_handshake_discarded(false)
        .prepare(&mut tcp_stream)
        .expect("should prepare TCP stream for IMAP STARTTLS");

    println!("connection TLS-ready, disconnecting…");
    tcp_stream
        .shutdown(Shutdown::Both)
        .expect("should close TCP stream");
}