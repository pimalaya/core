#![cfg(feature = "pgp-gpg")]

use std::path::PathBuf;

#[cfg(feature = "async-std")]
use async_std::test;
use concat_with::concat_line;
use mml::{
    pgp::{Pgp, PgpGpg},
    MimeInterpreterBuilder, MmlCompilerBuilder,
};
#[cfg(feature = "tokio")]
use tokio::test;

#[test_log::test(test)]
async fn pgp_gpg() {
    let pgp = Pgp::Gpg(PgpGpg {
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
