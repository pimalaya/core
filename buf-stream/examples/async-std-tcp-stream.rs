#![cfg(feature = "async")]

use std::env;

use async_std::{
    io::{stdin, stdout},
    net::TcpStream,
};
use buf_stream::futures::BufStream;
use futures_util::{io::BufReader, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tracing::debug;

#[async_std::main]
async fn main() {
    env_logger::builder().is_test(true).init();

    let mut buf = [0; 512];
    let mut line = String::new();

    let host = env::var("HOST").expect("HOST should be defined");
    let port: u16 = env::var("PORT")
        .expect("PORT should be defined")
        .parse()
        .expect("PORT should be an unsigned integer");

    println!("connecting to {host}:{port} using TCP…");
    let tcp_stream = TcpStream::connect((host.as_str(), port))
        .await
        .expect("should connect to TCP stream");
    let mut tcp_stream = BufStream::new(tcp_stream);
    println!("connected! waiting for first bytes…");

    tcp_stream
        .flush()
        .await
        .expect("should receive first bytes from buf stream");

    line.clear();
    while tcp_stream.wants_read() {
        let count = tcp_stream
            .read(&mut buf)
            .await
            .expect("should read first bytes");
        let chunk = String::from_utf8_lossy(&buf[..count]);
        debug!("output chunk: {chunk:?}");
        line += &chunk;
    }
    println!("output: {line:?}");

    loop {
        println!();
        print!("prompt> ");
        stdout().flush().await.expect("should flush stdout");

        line.clear();
        BufReader::new(stdin())
            .read_line(&mut line)
            .await
            .expect("should read line from stdin");

        tcp_stream
            .write(line.trim_end().as_bytes())
            .await
            .expect("should write line to buffered stream");
        tcp_stream
            .write(b"\r\n")
            .await
            .expect("should write line to buffered stream");

        tcp_stream
            .flush()
            .await
            .expect("should flush buffered stream");

        line.clear();
        while tcp_stream.wants_read() {
            let count = tcp_stream
                .read(&mut buf)
                .await
                .expect("should read first bytes");
            let chunk = String::from_utf8_lossy(&buf[..count]);
            debug!("output chunk: {chunk:?}");
            line += &chunk;
        }
        println!("output: {line:?}");
    }
}
