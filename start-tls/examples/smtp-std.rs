#![cfg(feature = "blocking")]

use std::{
    env,
    net::{Shutdown, TcpStream},
};

use start_tls::{smtp::SmtpStartTls, StartTls};

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

    StartTls::new(SmtpStartTls::new(&mut tcp_stream))
        .prepare()
        .expect("should prepare TCP stream for SMTP STARTTLS");

    println!("disconnecting…");
    tcp_stream
        .shutdown(Shutdown::Both)
        .expect("should close TCP stream");
}
