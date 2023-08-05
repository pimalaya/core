#[cfg(feature = "cmds-pgp")]
#[tokio::test]
async fn cmds_pgp() {
    use concat_with::concat_line;
    use pimalaya_email_tpl::{CmdsPgp, Pgp, Tpl, TplInterpreter};
    use pimalaya_process::Cmd;

    env_logger::builder().is_test(true).init();

    let pgp = Pgp::Cmds(CmdsPgp {
        encrypt_cmd: Some(Cmd::from(
            "gpg --homedir ./tests/gpg-home -eqa <recipients>",
        )),
        encrypt_recipient_fmt: Some(CmdsPgp::default_encrypt_recipient_fmt()),
        encrypt_recipients_sep: Some(CmdsPgp::default_encrypt_recipients_sep()),
        decrypt_cmd: Some(Cmd::from("gpg --homedir ./tests/gpg-home -dq")),
        sign_cmd: Some(Cmd::from("gpg --homedir ./tests/gpg-home -saq")),
        verify_cmd: Some(Cmd::from("gpg --homedir ./tests/gpg-home --verify -q")),
    });

    let tpl = Tpl::from(concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "<#part type=text/plain encrypt=pgpmime sign=pgpmime>",
        "Encrypted and signed message!",
    ));

    let builder = tpl.with_pgp(pgp.clone()).compile().await.unwrap();

    let tpl = TplInterpreter::new()
        .with_show_only_headers(["From", "To", "Subject"])
        .with_pgp(pgp.clone())
        .interpret_msg_builder(builder)
        .await
        .unwrap();

    let expected_tpl = Tpl::from(concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "Encrypted and signed message!",
        ""
    ));

    assert_eq!(tpl, expected_tpl);
}
