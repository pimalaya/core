use std::{thread, time::Duration};

use pimalaya::time::pomodoro::{
    client::{Client, TcpClient},
    server::{Server, TcpServer},
    timer::{Cycle, State, Timer},
};

#[test]
fn time_pomodoro() {
    env_logger::builder().is_test(true).init();

    let addr = "127.0.0.1:3000";
    let server = TcpServer::new(addr);
    let mut client1 = TcpClient::new(addr);
    let mut client2 = TcpClient::new(addr);

    let process = server.start().unwrap();

    client1.start().unwrap();
    thread::sleep(Duration::from_secs(2));

    assert_eq!(
        client1.get().unwrap(),
        Timer {
            state: State::Running,
            cycle: Cycle::Work1,
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
            state: State::Paused,
            cycle: Cycle::Work1,
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
            state: State::Running,
            cycle: Cycle::Work1,
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
            state: State::Stopped,
            cycle: Cycle::Work1,
            value: 1500,
            work_duration: 1500,
            short_break_duration: 300,
            long_break_duration: 900,
        }
    );

    server.stop().unwrap();
    process.join().unwrap();
}
