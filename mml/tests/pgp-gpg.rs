#[cfg(feature = "pgp-gpg")]
#[tokio::test]
async fn pgp_gpg() {
    use concat_with::concat_line;
    use mml::{Gpg, MimeInterpreter, MmlCompiler, Pgp};
    use std::path::PathBuf;

    env_logger::builder().is_test(true).init();

    let pgp = Pgp::Gpg(Gpg {
        home_dir: Some(PathBuf::from("./tests/gpg-home")),
    });

    let mml = concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "<#part type=text/plain encrypt=pgpmime sign=pgpmime>",
        "Encrypted and signed message!",
    );

    let msg_builder = MmlCompiler::new()
        .with_pgp(pgp.clone())
        .compile(mml)
        .await
        .unwrap();

    let mml = MimeInterpreter::new()
        .with_show_only_headers(["From", "To", "Subject"])
        .with_pgp(pgp.clone())
        .interpret_msg_builder(msg_builder)
        .await
        .unwrap();

    let expected_mml = concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "Encrypted and signed message!",
        ""
    );

    assert_eq!(mml, expected_mml);
}
