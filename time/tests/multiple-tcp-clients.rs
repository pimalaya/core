use std::time::Duration;

#[cfg(feature = "async-std")]
use async_std::{task::sleep, test};
use time::{
    client::tcp::TcpClient,
    server::{tcp::TcpBind, ServerBuilder, ServerEvent},
    timer::{Timer, TimerCycle, TimerEvent, TimerState},
};
#[cfg(feature = "tokio")]
use tokio::{test, time::sleep};

static HOST: &str = "127.0.0.1";
static PORT: u16 = 1234;

#[test_log::test(test)]
async fn multiple_tcp_clients() {
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
            sleep(Duration::from_secs(1)).await;

            let client1 = TcpClient::new_boxed(HOST, PORT);
            let client2 = TcpClient::new_boxed(HOST, PORT);

            client1.start().await.unwrap();
            sleep(Duration::from_secs(2)).await;

            assert_eq!(
                client1.get().await.unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::new("Work", 1),
                    ..Timer::default()
                }
            );

            client1.pause().await.unwrap();
            sleep(Duration::from_secs(2)).await;

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
            sleep(Duration::from_secs(2)).await;

            assert_eq!(
                client1.get().await.unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::new("Break", 5),
                    elapsed: 2,
                    ..Timer::default()
                }
            );

            sleep(Duration::from_secs(2)).await;

            assert_eq!(
                client1.get().await.unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::new("Break", 3),
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
