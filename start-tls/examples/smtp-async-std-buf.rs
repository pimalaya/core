#![cfg(feature = "async")]

use std::env;

use async_std::net::TcpStream;
use buf_stream::futures::BufStream;
use futures_util::{AsyncReadExt, AsyncWriteExt};
use start_tls::{smtp::SmtpStartTls, StartTlsExt};

const READ_BUF_CAPACITY: usize = 1024;

#[async_std::main]
async fn main() {
    env_logger::builder().is_test(true).init();

    let host = env::var("HOST").expect("HOST should be defined");
    let port: u16 = env::var("PORT")
        .expect("PORT should be defined")
        .parse()
        .expect("PORT should be an unsigned integer");

    println!("connecting to {host}:{port} using buffered TCP…");
    let tcp_stream = TcpStream::connect((host.as_str(), port))
        .await
        .expect("should connect to buffered TCP stream");
    let mut tcp_stream_buffered = BufStream::new(tcp_stream).with_read_capacity(READ_BUF_CAPACITY);

    println!("preparing buffered TCP connection for STARTTLS…");
    SmtpStartTls::new()
        .with_read_buffer_capacity(READ_BUF_CAPACITY)
        .with_handshake_discarded(true)
        .prepare(&mut tcp_stream_buffered)
        .await
        .expect("should prepare buffered TCP stream for SMTP STARTTLS");

    tcp_stream_buffered
        .progress()
        .await
        .expect("should sync buffered TCP stream");

    let mut discarded = String::new();
    tcp_stream_buffered
        .read_to_string(&mut discarded)
        .await
        .expect("should read discarded lines from buffered TCP stream");
    println!("discarded lines: {discarded:?}");

    println!("connection TLS-ready, disconnecting…");
    tcp_stream_buffered
        .close()
        .await
        .expect("should close buffered TCP stream");
}
