#[cfg(feature = "gpg")]
#[tokio::test]
async fn gpg() {
    use concat_with::concat_line;
    use pimalaya_email_tpl::{Gpg, Pgp, Tpl, TplInterpreter};
    use std::path::PathBuf;

    env_logger::builder().is_test(true).init();

    let pgp = Pgp::Gpg(Gpg {
        home_dir: Some(PathBuf::from("./tests/gpg-home")),
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
