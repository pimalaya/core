#![cfg(feature = "pgp-commands")]

use concat_with::concat_line;
use mml::{
    pgp::{CmdsPgp, Pgp},
    MimeInterpreterBuilder, MmlCompilerBuilder,
};
use process::Command;

#[tokio::test]
async fn pgp_cmds() {
    env_logger::builder().is_test(true).init();

    let pgp = Pgp::Cmds(CmdsPgp {
        encrypt_cmd: Some(Command::from(
            "gpg --homedir ./tests/gpg-home -eqa <recipients>",
        )),
        encrypt_recipient_fmt: Some(CmdsPgp::default_encrypt_recipient_fmt()),
        encrypt_recipients_sep: Some(CmdsPgp::default_encrypt_recipients_sep()),
        decrypt_cmd: Some(Command::from("gpg --homedir ./tests/gpg-home -dq")),
        sign_cmd: Some(Command::from("gpg --homedir ./tests/gpg-home -saq")),
        verify_cmd: Some(Command::from("gpg --homedir ./tests/gpg-home --verify -q")),
    });

    let mml = concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "<#part type=text/plain encrypt=pgpmime sign=pgpmime>",
        "Encrypted and signed message!",
        "<#/part>",
    );

    let mml_compiler = MmlCompilerBuilder::new()
        .with_pgp(pgp.clone())
        .build(mml)
        .unwrap();
    let msg_builder = mml_compiler.compile().await.unwrap().into_msg_builder();

    let mml = MimeInterpreterBuilder::new()
        .with_show_only_headers(["From", "To", "Subject"])
        .with_pgp(pgp.clone())
        .build()
        .from_msg_builder(msg_builder)
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
