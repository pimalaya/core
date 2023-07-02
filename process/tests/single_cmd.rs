use pimalaya_process::{Cmd, Error};

#[tokio::test]
async fn single_cmd() {
    env_logger::builder().is_test(true).init();

    let cmd = Cmd::from("echo <msg>1 <msg>2").replace("<msg>", "hello");
    let out = cmd.run().await.unwrap().to_string_lossy();
    assert_eq!(out, "hello1 hello2\n");

    match Cmd::from("bad").run().await.unwrap_err() {
        Error::InvalidExitStatusCodeNonZeroError(cmd, status, err) => {
            assert_eq!(cmd, "bad");
            assert_eq!(status, 127);
            assert_eq!(err, "sh: line 1: bad: command not found\n");
        }
        err => panic!("unexpected error: {err:?}"),
    }
}
