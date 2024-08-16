#![cfg(feature = "pgp-gpg")]

use concat_with::concat_line;
use mml::{
    pgp::{Gpg, Pgp},
    MimeInterpreterBuilder, MmlCompilerBuilder,
};
use std::path::PathBuf;

#[tokio::test]
async fn pgp_gpg() {
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
