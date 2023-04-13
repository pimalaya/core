use pimalaya::time::pomodoro::{ServerBuilder, ServerEvent, TcpBind, TcpClient, TimerEvent};
use std::{thread, time::Duration};

const HOST: &str = "127.0.0.1";
const PORT: u16 = 3000;

pub fn main() {
    let server = ServerBuilder::new()
        .with_server_handler(|event: ServerEvent| {
            println!("server event: {:?}", event);
            Ok(())
        })
        .with_timer_handler(|event: TimerEvent| {
            println!("timer event: {:?}", event);
            Ok(())
        })
        .with_binder(TcpBind::new(HOST, PORT))
        .build();

    server
        .bind_with(|| {
            let client = TcpClient::new(HOST, PORT);
            client.start().unwrap();
            thread::sleep(Duration::from_secs(3));
            client.pause().unwrap();
            thread::sleep(Duration::from_secs(3));
            let timer = client.get().unwrap();
            println!("current timer: {:?}", timer);
            Ok(())
        })
        .unwrap();
}
