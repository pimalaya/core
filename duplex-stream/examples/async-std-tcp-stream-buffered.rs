#![cfg(feature = "async")]

use async_std::{
    io::{stdin, stdout},
    net::TcpStream,
};
use duplex_stream::r#async::DuplexStream;
use futures::{io::BufReader, AsyncBufReadExt, AsyncWriteExt};

#[test_log::test(async_std::main)]
async fn main() {
    let host = std::env::var("HOST").expect("HOST should be defined");
    let port: u16 = std::env::var("PORT")
        .expect("PORT should be defined")
        .parse()
        .expect("PORT should be an unsigned integer");

    println!("connecting to {host}:{port} using TCP…");
    let tcp_stream = TcpStream::connect((host.as_str(), port))
        .await
        .expect("should connect to TCP stream");
    let mut tcp_stream = DuplexStream::new(tcp_stream);
    println!("connected! waiting for first bytes…");

    let count = tcp_stream
        .progress_read()
        .await
        .expect("should receive first bytes from stream");
    let bytes = String::from_utf8_lossy(&tcp_stream.read_buffer()[..count]);
    println!("buffered output: {bytes:?}");

    loop {
        println!();
        print!("prompt> ");
        stdout().flush().await.expect("should flush stdout");

        let mut line = String::new();
        BufReader::new(stdin())
            .read_line(&mut line)
            .await
            .expect("should read line from stdin");

        tcp_stream.push_bytes(line.trim_end().as_bytes());
        tcp_stream.push_bytes(b"\r\n");
        let bytes = tcp_stream
            .write_buffer()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        println!("buffered input: {:?}", String::from_utf8_lossy(&bytes));

        let bytes = tcp_stream
            .progress()
            .await
            .expect("should progress duplex stream");
        println!("buffered output: {:?}", String::from_utf8_lossy(bytes));
    }
}
