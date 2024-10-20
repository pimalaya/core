#[cfg(feature = "async-std")]
use async_std::test;
use process::{Error, Pipeline};
#[cfg(feature = "tokio")]
use tokio::test;

#[test_log::test(test)]
async fn test_pipeline() {
    let cmd = Pipeline::new(vec!["echo hello", "cat"]);
    let out = cmd.run().await.unwrap().to_string_lossy();
    assert_eq!(out, "hello\n");

    let cmd = Pipeline::new(vec!["echo hello", "bad", "cat"]);
    match cmd.run().await.unwrap_err() {
        Error::GetExitStatusCodeNonZeroError(cmd, status, err) => {
            assert_eq!(cmd, "bad");
            assert_eq!(status, 127);
            assert_eq!(err, "sh: line 1: bad: command not found\n");
        }
        err => panic!("unexpected error: {err:?}"),
    }
}
