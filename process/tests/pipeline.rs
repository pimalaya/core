use pimalaya_process::{Cmd, Error};

#[tokio::test]
async fn pipeline() {
    env_logger::builder().is_test(true).init();

    let cmd = Cmd::from(vec!["echo hello", "cat"]);
    let out = cmd.run().await.unwrap().to_string_lossy();

    assert_eq!(out, "hello\n");

    let cmd = Cmd::from(vec!["echo hello", "bad", "cat"]);
    match cmd.run().await.unwrap_err() {
        Error::InvalidExitStatusCodeNonZeroError(cmd, status, err) => {
            assert_eq!(cmd, "bad");
            assert_eq!(status, 127);
            assert_eq!(err, "sh: line 1: bad: command not found\n");
        }
        err => panic!("unexpected error: {err:?}"),
    }
}
