use pimalaya_process::Cmd;

#[tokio::test]
async fn pipeline() {
    env_logger::builder().is_test(true).init();

    let cmd = Cmd::from(vec!["echo hello", "cat"]);
    let res = cmd.run().await.unwrap();

    assert_eq!(res.code, 0);
    assert_eq!(res.read_out_lossy(), "hello\n");

    let cmd = Cmd::from(vec!["echo hello", "bad", "cat"]);
    let res = cmd.run().await.unwrap();

    assert_eq!(res.code, 127);
    assert_eq!(res.read_out_lossy(), "sh: line 1: bad: command not found\n");
}
