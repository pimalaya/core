mod client;
mod request;
mod response;
mod server;
mod timer;

pub(crate) use request::Request;
pub(crate) use response::Response;

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::{
        client::{Client, TcpClient},
        server::{Server, TcpServer},
    };

    #[test]
    fn start() {
        let server = TcpServer::new("127.0.0.1:3000").unwrap();
        let mut client1 = TcpClient::new("127.0.0.1:3000");
        let mut client2 = TcpClient::new("127.0.0.1:3000");

        let p = thread::spawn(move || {
            server.bind().unwrap();
        });

        client1.start().unwrap();
        thread::sleep(Duration::from_secs(2));

        let timer = client1.get().unwrap();
        println!("client 1 timer: {timer}");

        let timer = client2.get().unwrap();
        println!("client 2 timer: {timer}");

        client2.stop().unwrap();
        thread::sleep(Duration::from_secs(2));

        client2.kill().unwrap();

        p.join().unwrap();
    }
}
