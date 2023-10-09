use std::{thread, time::Duration};
use time::{ServerBuilder, ServerEvent, TcpBind, TcpClient, TimerEvent};

const HOST: &str = "127.0.0.1";
const PORT: u16 = 3000;

fn main() {
    let server = ServerBuilder::new()
        .with_server_handler(|event: ServerEvent| {
            println!("server event: {event:?}");
            Ok(())
        })
        .with_timer_handler(|event: TimerEvent| {
            println!("timer event: {event:?}");
            Ok(())
        })
        .with_binder(TcpBind::new(HOST, PORT))
        .with_pomodoro_config()
        .build()
        .unwrap();

    server
        .bind_with(|| {
            // wait for the binder to be ready
            thread::sleep(Duration::from_secs(1));

            let client = TcpClient::new(HOST, PORT);

            client.start().unwrap();
            thread::sleep(Duration::from_secs(1));

            client.pause().unwrap();
            thread::sleep(Duration::from_secs(1));

            let timer = client.get().unwrap();
            println!("current timer: {timer:?}");

            Ok(())
        })
        .unwrap();
}
