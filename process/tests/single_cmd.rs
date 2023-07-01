use pimalaya_process::Cmd;

#[tokio::test]
async fn single_cmd() {
    env_logger::builder().is_test(true).init();

    let cmd = Cmd::from("echo <msg> <msg>").replace("<msg>", "hello");
    let res = cmd.run().await.unwrap();

    assert_eq!(res.code, 0);
    assert_eq!(res.read_out().unwrap(), "hello hello\n");

    let cmd = Cmd::from("bad");
    let res = cmd.run().await.unwrap();

    assert_eq!(res.code, 127);
    assert_eq!(
        res.read_out().unwrap(),
        "sh: line 1: bad: command not found\n"
    );
}
