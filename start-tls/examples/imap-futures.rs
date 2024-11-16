#![cfg(feature = "async")]

use std::env;

use async_std::net::TcpStream;
use futures::AsyncWriteExt;
use start_tls::{imap::ImapStartTls, StartTls};

#[async_std::main]
async fn main() {
    env_logger::builder().is_test(true).init();

    let host = env::var("HOST").expect("HOST should be defined");
    let port: u16 = env::var("PORT")
        .expect("PORT should be defined")
        .parse()
        .expect("PORT should be an unsigned integer");

    println!("connecting to {host}:{port} using TCP…");
    let mut tcp_stream = TcpStream::connect((host.as_str(), port))
        .await
        .expect("should connect to TCP stream");

    let spec = ImapStartTls::new(&mut tcp_stream);
    StartTls::new(spec)
        .prepare()
        .await
        .expect("should prepare TCP stream for IMAP STARTTLS");

    println!("disconnecting…");
    tcp_stream.close().await.expect("should close TCP stream");
}
