#![cfg(feature = "blocking")]

use std::{
    env,
    net::{Shutdown, TcpStream},
};

use start_tls::{imap::ImapStartTls, StartTls};

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

    let spec = ImapStartTls::new(&mut tcp_stream);
    StartTls::new(spec)
        .prepare()
        .expect("should prepare TCP stream for IMAP STARTTLS");

    println!("disconnecting…");
    tcp_stream
        .shutdown(Shutdown::Both)
        .expect("should close TCP stream");
}
