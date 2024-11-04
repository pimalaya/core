#![cfg(feature = "tls")]

use std::{
    env,
    io::{Read, Write},
    sync::Arc,
};

use byte_string::ByteStr;
use rustls_platform_verifier::ConfigVerifierExt;
use compote::{tcp::TcpStream, tls::TlsStream};
use tokio::io::{stdin, AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() {
    let host = env::var("HOST").unwrap_or(String::from("www.rust-lang.org"));
    let host = host.as_str();

    println!("This example will connect to {host}");

    let tls: String = loop {
        println!("\nPlease enter rustls|tokio-rustls|native-tls|tokio-native-tls:");

        let mut input = String::new();
        BufReader::new(stdin()).read_line(&mut input).await.unwrap();

        match input.trim() {
            "rustls" | "tokio-rustls" | "native-tls" | "tokio-native-tls" => {
                break input.trim().to_owned()
            }
            _ => {
                continue;
            }
        }
    };

    let tcp_stream: TcpStream = loop {
        println!("\nPlease enter std|tokio:");

        let mut input = String::new();
        BufReader::new(stdin()).read_line(&mut input).await.unwrap();

        match input.trim() {
            "std" => {
                break TcpStream::std_connect((host, 443)).unwrap();
            }
            "tokio" => {
                break TcpStream::tokio_connect((host, 443)).await.unwrap();
            }
            _ => {
                continue;
            }
        }
    };

    let mut tls_stream = match tls.as_str() {
        "rustls" => {
            let srv_name = host.to_owned().try_into().unwrap();
            let tls_config = Arc::new(rustls::ClientConfig::with_platform_verifier());
            let tls_connection = rustls::client::ClientConnection::new(tls_config, srv_name);
            let tls_stream = rustls::StreamOwned::new(tls_connection.unwrap(), tcp_stream);
            TlsStream::from(tls_stream)
        }
        "native-tls" => {
            let connector = tokio_native_tls::native_tls::TlsConnector::new().unwrap();
            let connector = tokio_native_tls::TlsConnector::from(connector);
            let tls_stream = connector.connect(host, tcp_stream).await.unwrap();
            TlsStream::from(tls_stream)
        }
        "tokio-rustls" => {
            let srv_name = host.to_owned().try_into().unwrap();
            let tls_config = Arc::new(rustls::ClientConfig::with_platform_verifier());
            let tls_connector = tokio_rustls::TlsConnector::from(tls_config);
            let tls_stream = tls_connector.connect(srv_name, tcp_stream).await.unwrap();
            TlsStream::from(tls_stream)
        }
        "tokio-native-tls" => {
            let connector = tokio_native_tls::native_tls::TlsConnector::new().unwrap();
            let connector = tokio_native_tls::TlsConnector::from(connector);
            let tls_stream = connector.connect(host, tcp_stream).await.unwrap();
            TlsStream::from(tls_stream)
        }
        _ => unreachable!(),
    };

    let content = format!("GET / HTTP/1.0\r\nHost: {host}\r\n\r\n");
    tls_stream.write_all(content.as_bytes()).unwrap();
    println!("\nSent: {content:?}");

    let mut plaintext = Vec::new();
    tls_stream.read_to_end(&mut plaintext).unwrap();
    println!("\nReceived: {:?}", ByteStr::new(&plaintext));
}
