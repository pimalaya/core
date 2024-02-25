use std::time::Duration;
use time::{
    client::tcp::TcpClient,
    server::{tcp::TcpBind, ServerBuilder, ServerEvent},
    timer::TimerEvent,
};

static HOST: &str = "127.0.0.1";
static PORT: u16 = 3000;

#[tokio::main]
async fn main() {
    let server = ServerBuilder::new()
        .with_server_handler(|event: ServerEvent| async move {
            println!("server event: {event:?}");
            Ok(())
        })
        .with_timer_handler(|event: TimerEvent| async move {
            println!("timer event: {event:?}");
            Ok(())
        })
        .with_binder(TcpBind::new(HOST, PORT))
        .with_pomodoro_config()
        .build()
        .unwrap();

    server
        .bind_with(|| async {
            // wait for the binder to be ready
            tokio::time::sleep(Duration::from_secs(1)).await;

            let client = TcpClient::new_boxed(HOST, PORT);

            client.start().await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;

            client.pause().await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;

            let timer = client.get().await.unwrap();
            println!("current timer: {timer:?}");

            Ok(())
        })
        .await
        .unwrap();
}
