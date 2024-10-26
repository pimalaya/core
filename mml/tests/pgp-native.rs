#![cfg(feature = "pgp-native")]

use std::collections::HashMap;

#[cfg(feature = "async-std")]
use async_std::test;
use concat_with::concat_line;
use mml::{
    pgp::{NativePgpPublicKeysResolver, NativePgpSecretKey, Pgp, PgpNative},
    MimeInterpreterBuilder, MmlCompilerBuilder,
};
use pgp::gen_key_pair;
use secret::Secret;
use tempfile::tempdir;
#[cfg(feature = "tokio")]
use tokio::test;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task,
};

#[test_log::test(test)]
async fn pgp_native() {
    async fn spawn_fake_key_server(pkeys: HashMap<String, String>) -> String {
        let listener = TcpListener::bind(("localhost", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let uri = format!("http://localhost:{port}/<email>");

        task::spawn(async move {
            loop {
                println!("waiting for request…");
                let (mut stream, _) = listener.accept().await.unwrap();
                println!("incomming request!");

                let mut reader = BufReader::new(&mut stream);
                println!("reader!");

                let mut http_req = String::new();
                reader.read_line(&mut http_req).await.unwrap();
                let email = &http_req.split_whitespace().take(2).last().unwrap()[1..];
                match pkeys.get(email) {
                    Some(pkey) => {
                        let res = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{pkey}",
                            pkey.len(),
                        );
                        stream.write_all(res.as_bytes()).await.unwrap();
                    }
                    None => {
                        stream.write_all(b"HTTP/1.1 404 Not Found").await.unwrap();
                    }
                }
            }
        });

        uri
    }

    let dir = tempdir().unwrap();

    let (alice_skey, alice_pkey) = gen_key_pair("alice@localhost", "").await.unwrap();
    let alice_skey_path = dir.path().join("alice.key");
    fs::write(&alice_skey_path, alice_skey.to_armored_bytes(None).unwrap())
        .await
        .unwrap();

    let (bob_skey, bob_pkey) = gen_key_pair("bob@localhost", "").await.unwrap();
    let bob_skey_path = dir.path().join("bob.key");
    fs::write(&bob_skey_path, bob_skey.to_armored_bytes(None).unwrap())
        .await
        .unwrap();

    let key_server_addr = spawn_fake_key_server(HashMap::from_iter([
        (
            String::from("alice@localhost"),
            alice_pkey.to_armored_string(None).unwrap(),
        ),
        (
            String::from("bob@localhost"),
            bob_pkey.to_armored_string(None).unwrap(),
        ),
    ]))
    .await;

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
        .with_pgp(Pgp::Native(PgpNative {
            secret_key: NativePgpSecretKey::Path(alice_skey_path.clone()),
            secret_key_passphrase: Secret::new_raw(""),
            public_keys_resolvers: vec![NativePgpPublicKeysResolver::KeyServers(vec![
                key_server_addr,
            ])],
        }))
        .build(mml)
        .unwrap();
    let msg_builder = mml_compiler.compile().await.unwrap().into_msg_builder();

    let mml = MimeInterpreterBuilder::new()
        .with_show_only_headers(["From", "To", "Subject"])
        .with_pgp(Pgp::Native(PgpNative {
            secret_key: NativePgpSecretKey::Raw(bob_skey.clone()),
            secret_key_passphrase: Secret::new_raw(""),
            public_keys_resolvers: vec![NativePgpPublicKeysResolver::Raw(
                "alice@localhost".into(),
                alice_pkey.clone(),
            )],
        }))
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
