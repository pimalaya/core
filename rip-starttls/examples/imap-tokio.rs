#![cfg(feature = "tokio")]

use std::env;

use rip_starttls::imap::tokio::RipStarttls;
use tokio::{io::AsyncWriteExt, net::TcpStream};

#[tokio::main]
async fn main() {
    env_logger::builder().is_test(true).init();

    let host = env::var("HOST").expect("HOST should be defined");
    let port: u16 = env::var("PORT")
        .expect("PORT should be defined")
        .parse()
        .expect("PORT should be an unsigned integer");

    println!("connecting to {host}:{port} using TCP…");
    let tcp_stream = TcpStream::connect((host.as_str(), port))
        .await
        .expect("should connect to TCP stream");

    println!("preparing TCP connection for STARTTLS…");
    let mut tcp_stream = RipStarttls::default()
        .do_starttls_prefix(tcp_stream)
        .await
        .expect("should prepare TCP stream for IMAP STARTTLS");

    println!("connection TLS-ready, disconnecting…");
    tcp_stream
        .shutdown()
        .await
        .expect("should close TCP stream");
}
