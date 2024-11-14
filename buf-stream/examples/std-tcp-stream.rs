#![cfg(feature = "blocking")]

use buf_stream::std::BufStream;
use std::{
    io::{stdin, stdout, BufRead, BufReader, Write},
    net::TcpStream,
};

fn main() {
    env_logger::builder().is_test(true).init();

    let host = std::env::var("HOST").expect("HOST should be defined");
    let port: u16 = std::env::var("PORT")
        .expect("PORT should be defined")
        .parse()
        .expect("PORT should be an unsigned integer");

    println!("connecting to {host}:{port} using TCP…");
    let tcp_stream =
        TcpStream::connect((host.as_str(), port)).expect("should connect to TCP stream");
    let mut tcp_stream = BufStream::new(tcp_stream);
    println!("connected! waiting for first bytes…");

    let count = tcp_stream
        .progress_read()
        .expect("should receive first bytes from buf stream");
    let bytes = &tcp_stream.read_buffer()[..count];
    println!("buffered output: {:?}", String::from_utf8_lossy(bytes));

    loop {
        println!();
        print!("prompt> ");
        stdout().flush().expect("should flush stdout");

        let mut line = String::new();
        BufReader::new(stdin())
            .read_line(&mut line)
            .expect("should read line from stdin");

        tcp_stream.push_bytes(line.trim_end().as_bytes());
        tcp_stream.push_bytes(b"\r\n");
        let bytes = tcp_stream
            .write_buffer()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        println!("buffered input: {:?}", String::from_utf8_lossy(&bytes));

        let bytes = tcp_stream.progress().expect("should progress buf stream");
        println!("buffered output: {:?}", String::from_utf8_lossy(bytes));
    }
}
