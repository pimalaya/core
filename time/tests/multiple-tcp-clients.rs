use std::time::Duration;
use time::{
    client::tcp::TcpClient,
    server::{tcp::TcpBind, ServerBuilder, ServerEvent},
    timer::{Timer, TimerCycle, TimerEvent, TimerState},
};

static HOST: &str = "127.0.0.1";
static PORT: u16 = 3000;

#[tokio::test(flavor = "multi_thread")]
async fn multiple_tcp_clients() {
    env_logger::builder().is_test(true).init();

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
        .with_cycle(("Work", 3))
        .with_cycle(("Break", 5))
        .build()
        .unwrap();

    server
        .bind_with(|| async {
            // wait for the binder to be ready
            tokio::time::sleep(Duration::from_secs(1)).await;

            let client1 = TcpClient::new(HOST, PORT);
            let client2 = TcpClient::new(HOST, PORT);

            client1.start().await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;

            assert_eq!(
                client1.get().await.unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::new("Work", 1),
                    ..Timer::default()
                }
            );

            client1.pause().await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;

            assert_eq!(
                client2.get().await.unwrap(),
                Timer {
                    state: TimerState::Paused,
                    cycle: TimerCycle::new("Work", 1),
                    elapsed: 2,
                    ..Timer::default()
                }
            );

            client1.resume().await.unwrap();
            tokio::time::sleep(Duration::from_secs(2)).await;

            assert_eq!(
                client1.get().await.unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::new("Break", 4),
                    elapsed: 2,
                    ..Timer::default()
                }
            );

            tokio::time::sleep(Duration::from_secs(2)).await;

            assert_eq!(
                client1.get().await.unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::new("Break", 2),
                    elapsed: 2,
                    ..Timer::default()
                }
            );

            client2.stop().await.unwrap();

            assert_eq!(
                client2.get().await.unwrap(),
                Timer {
                    state: TimerState::Stopped,
                    cycle: TimerCycle::new("Work", 3),
                    ..Timer::default()
                }
            );

            Ok(())
        })
        .await
        .unwrap();
}
