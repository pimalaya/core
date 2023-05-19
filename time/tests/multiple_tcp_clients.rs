use std::{thread, time::Duration};

use pimalaya_time::{
    ServerBuilder, ServerEvent, TcpBind, TcpClient, Timer, TimerCycle, TimerEvent, TimerState,
};

#[test]
fn multiple_tcp_clients() {
    env_logger::builder().is_test(true).init();

    let host = "127.0.0.1";
    let port = 3000;
    let server = ServerBuilder::new()
        .with_server_handler(|event: ServerEvent| {
            println!("server event: {:?}", event);
            Ok(())
        })
        .with_timer_handler(|event: TimerEvent| {
            println!("timer event: {:?}", event);
            Ok(())
        })
        .with_binder(TcpBind::new(host, port))
        .with_cycle(("Work", 3))
        .with_cycle(("Break", 5))
        .build()
        .unwrap();

    server
        .bind_with(|| {
            let client1 = TcpClient::new(host, port);
            let client2 = TcpClient::new(host, port);

            client1.start().unwrap();
            thread::sleep(Duration::from_secs(2));

            assert_eq!(
                client1.get().unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::new("Work", 1),
                    ..Timer::default()
                }
            );

            client1.pause().unwrap();
            thread::sleep(Duration::from_secs(2));

            assert_eq!(
                client2.get().unwrap(),
                Timer {
                    state: TimerState::Paused,
                    cycle: TimerCycle::new("Work", 1),
                    ..Timer::default()
                }
            );

            client1.resume().unwrap();
            thread::sleep(Duration::from_secs(2));

            assert_eq!(
                client1.get().unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::new("Break", 4),
                    ..Timer::default()
                }
            );

            client2.stop().unwrap();

            assert_eq!(
                client2.get().unwrap(),
                Timer {
                    state: TimerState::Stopped,
                    cycle: TimerCycle::new("Work", 3),
                    ..Timer::default()
                }
            );

            Ok(())
        })
        .unwrap();
}