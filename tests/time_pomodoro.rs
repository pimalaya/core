use std::{thread, time::Duration};

use pimalaya::time::pomodoro::{Server, TcpBind, TcpClient, Timer, TimerCycle, TimerState};

#[test]
fn time_pomodoro() {
    env_logger::builder().is_test(true).init();

    let host = "127.0.0.1";
    let port = 3000;
    let server = Server::new([TcpBind::new(host, port)]);

    server
        .bind(|| {
            let client1 = TcpClient::new(host, port);
            let client2 = TcpClient::new(host, port);

            client1.start().unwrap();
            thread::sleep(Duration::from_secs(2));

            assert_eq!(
                client1.get().unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::Work1,
                    value: 1498,
                    work_duration: 1500,
                    short_break_duration: 300,
                    long_break_duration: 900,
                }
            );

            client1.pause().unwrap();
            thread::sleep(Duration::from_secs(2));

            assert_eq!(
                client2.get().unwrap(),
                Timer {
                    state: TimerState::Paused,
                    cycle: TimerCycle::Work1,
                    value: 1498,
                    work_duration: 1500,
                    short_break_duration: 300,
                    long_break_duration: 900,
                }
            );

            client1.resume().unwrap();
            thread::sleep(Duration::from_secs(2));

            assert_eq!(
                client1.get().unwrap(),
                Timer {
                    state: TimerState::Running,
                    cycle: TimerCycle::Work1,
                    value: 1496,
                    work_duration: 1500,
                    short_break_duration: 300,
                    long_break_duration: 900,
                }
            );

            client2.stop().unwrap();

            assert_eq!(
                client2.get().unwrap(),
                Timer {
                    state: TimerState::Stopped,
                    cycle: TimerCycle::Work1,
                    value: 1500,
                    work_duration: 1500,
                    short_break_duration: 300,
                    long_break_duration: 900,
                }
            );

            Ok(())
        })
        .unwrap();
}
