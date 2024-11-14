#![cfg(feature = "async")]

use async_std::{
    io::{stdin, stdout},
    net::TcpStream,
};
use duplex_stream::r#async::DuplexStream;
use futures::{io::BufReader, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

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

    let mut input_buf = [0; 1024];
    let mut output_buf;

    let count = tcp_stream
        .read(&mut input_buf)
        .await
        .expect("should receive first bytes from duplex stream");
    let bytes = &input_buf[..count];
    println!("output: {:?}", &String::from_utf8_lossy(bytes));

    loop {
        println!();
        print!("prompt> ");
        stdout().flush().await.expect("should flush stdout");

        let mut line = String::new();
        BufReader::new(stdin())
            .read_line(&mut line)
            .await
            .expect("should read line from stdin");

        output_buf = line.trim_end().to_owned() + "\r\n";
        tcp_stream
            .write(output_buf.as_bytes())
            .await
            .expect("should write line to duplex stream");
        println!("input: {output_buf:?}");

        let count = tcp_stream
            .read(&mut input_buf)
            .await
            .expect("should receive bytes from duplex stream");
        let bytes = &input_buf[..count];
        println!("output: {:?}", &String::from_utf8_lossy(bytes));
    }
}
