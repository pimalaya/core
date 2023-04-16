use std::{thread, time::Duration};

use pimalaya_pomodoro::{
    ServerBuilder, ServerEvent, TcpBind, TcpClient, Timer, TimerCycle, TimerEvent, TimerState,
};

#[test]
fn pomodoro() {
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
        .build();

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
                    cycle: TimerCycle::FirstWork,
                    value: 1498,
                    ..Timer::default()
                }
            );

            client1.pause().unwrap();
            thread::sleep(Duration::from_secs(2));

            assert_eq!(
                client2.get().unwrap(),
                Timer {
                    state: TimerState::Paused,
                    cycle: TimerCycle::FirstWork,
                    value: 1498,
                    ..Timer::default()
                }
            );

            client1.resume().unwrap();
            thread::sleep(Duration::from_secs(2));

            assert_eq!(
                client1.get().unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::FirstWork,
                    value: 1496,
                    ..Timer::default()
                }
            );

            client2.stop().unwrap();

            assert_eq!(
                client2.get().unwrap(),
                Timer {
                    state: TimerState::Stopped,
                    cycle: TimerCycle::FirstWork,
                    value: 1500,
                    ..Timer::default()
                }
            );

            Ok(())
        })
        .unwrap();
}
