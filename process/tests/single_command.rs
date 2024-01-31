use process::{Command, Error, SingleCommand};

#[tokio::test]
async fn test_single_command() {
    env_logger::builder().is_test(true).init();

    let cmd = SingleCommand::from("echo hello, world!");
    let out = cmd.run().await.unwrap().to_string_lossy();
    assert_eq!(out, "hello, world!\n");

    match Command::from("bad").run().await.unwrap_err() {
        Error::GetExitStatusCodeNonZeroError(cmd, status, err) => {
            assert_eq!(cmd, "bad");
            assert_eq!(status, 127);
            assert_eq!(err, "sh: line 1: bad: command not found\n");
        }
        err => panic!("unexpected error: {err:?}"),
    }
}
