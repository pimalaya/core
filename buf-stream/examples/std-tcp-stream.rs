#![cfg(feature = "blocking")]

use std::{
    env,
    io::{stdin, stdout, BufRead, BufReader, Read, Write},
    net::TcpStream,
};

use buf_stream::std::BufStream;
use tracing::debug;

fn main() {
    env_logger::builder().is_test(true).init();

    let mut buf = [0; 512];
    let mut line = String::new();

    let host = env::var("HOST").expect("HOST should be defined");
    let port: u16 = env::var("PORT")
        .expect("PORT should be defined")
        .parse()
        .expect("PORT should be an unsigned integer");

    println!("connecting to {host}:{port} using TCP…");
    let tcp_stream =
        TcpStream::connect((host.as_str(), port)).expect("should connect to TCP stream");
    let mut tcp_stream = BufStream::new(tcp_stream);
    println!("connected! waiting for first bytes…");

    tcp_stream
        .flush()
        .expect("should receive first bytes from buf stream");

    line.clear();
    while tcp_stream.wants_read() {
        let count = tcp_stream.read(&mut buf).expect("should read first bytes");
        let chunk = String::from_utf8_lossy(&buf[..count]);
        debug!("output chunk: {chunk:?}");
        line += &chunk;
    }
    println!("output: {line:?}");

    loop {
        println!();
        print!("prompt> ");
        stdout().flush().expect("should flush stdout");

        line.clear();
        BufReader::new(stdin())
            .read_line(&mut line)
            .expect("should read line from stdin");

        tcp_stream
            .write(line.trim_end().as_bytes())
            .expect("should write line to buffered stream");
        tcp_stream
            .write(b"\r\n")
            .expect("should write line to buffered stream");

        tcp_stream.flush().expect("should flush buffered stream");

        line.clear();
        while tcp_stream.wants_read() {
            let count = tcp_stream.read(&mut buf).expect("should read first bytes");
            let chunk = String::from_utf8_lossy(&buf[..count]);
            debug!("output chunk: {chunk:?}");
            line += &chunk;
        }
        println!("output: {line:?}");
    }
}
