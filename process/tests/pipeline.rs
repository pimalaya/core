use process::{Command, Error};

#[tokio::test]
async fn test_pipeline() {
    env_logger::builder().is_test(true).init();

    let cmd = Command::from(vec!["echo hello", "cat"]);
    let out = cmd.run().await.unwrap().to_string_lossy();
    assert_eq!(out, "hello\n");

    let cmd = Command::from("echo hello | cat");
    let out = cmd.run().await.unwrap().to_string_lossy();
    assert_eq!(out, "hello\n");

    let cmd = Command::from(vec!["echo hello", "bad", "cat"]);
    match cmd.run().await.unwrap_err() {
        Error::GetExitStatusCodeNonZeroError(cmd, status, err) => {
            assert_eq!(cmd, "bad");
            assert_eq!(status, 127);
            assert_eq!(err, "sh: line 1: bad: command not found\n");
        }
        err => panic!("unexpected error: {err:?}"),
    }
}
