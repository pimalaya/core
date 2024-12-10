#![cfg(feature = "pgp-commands")]

#[cfg(feature = "async-std")]
use async_std::test;
use concat_with::concat_line;
use mml::{
    pgp::{Pgp, PgpCommands},
    MimeInterpreterBuilder, MmlCompilerBuilder,
};
use process::Command;
#[cfg(feature = "tokio")]
use tokio::test;

#[test_log::test(test)]
async fn pgp_cmds() {
    let pgp = Pgp::Commands(PgpCommands {
        encrypt_cmd: Some(Command::new(
            "gpg --homedir ./tests/gpg-home -eqa <recipients>",
        )),
        encrypt_recipient_fmt: Some(PgpCommands::default_encrypt_recipient_fmt()),
        encrypt_recipients_sep: Some(PgpCommands::default_encrypt_recipients_sep()),
        decrypt_cmd: Some(Command::new("gpg --homedir ./tests/gpg-home -dq")),
        sign_cmd: Some(Command::new("gpg --homedir ./tests/gpg-home -saq")),
        verify_cmd: Some(Command::new("gpg --homedir ./tests/gpg-home --verify -q")),
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
