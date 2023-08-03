use concat_with::concat_line;
use pimalaya_email_tpl::{
    PgpPublicKey, PgpPublicKeyResolver, PgpPublicKeys, PgpPublicKeysResolver, PgpSecretKey,
    PgpSecretKeyResolver, Tpl, TplInterpreter,
};
use pimalaya_pgp::generate_key_pair;
use pimalaya_secret::Secret;
use std::collections::HashMap;
use tempfile::tempdir;
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    task,
};

#[tokio::test]
async fn pgp() {
    env_logger::builder().is_test(true).init();

    let dir = tempdir().unwrap();

    let (alice_skey, alice_pkey) = generate_key_pair("alice@localhost", "").await.unwrap();
    let alice_skey_path = dir.path().join("alice.key");
    fs::write(&alice_skey_path, alice_skey.to_armored_bytes(None).unwrap())
        .await
        .unwrap();

    let (bob_skey, bob_pkey) = generate_key_pair("bob@localhost", "").await.unwrap();
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

    let tpl = Tpl::from(concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "<#part type=text/plain encrypt=pgpmime sign=pgpmime>",
        "Encrypted and signed message!",
    ));

    let builder = tpl
        .with_pgp_encrypt(PgpPublicKeys::Enabled(vec![
            PgpPublicKeysResolver::KeyServers(vec![String::from(key_server_addr)]),
        ]))
        .with_pgp_sign(PgpSecretKey::Enabled(vec![PgpSecretKeyResolver::Path(
            alice_skey_path.clone(),
            Secret::new_raw(""),
        )]))
        .compile()
        .await
        .unwrap();

    let tpl = TplInterpreter::new()
        .with_show_only_headers(["From", "To", "Subject"])
        .with_pgp_decrypt(PgpSecretKey::Enabled(vec![PgpSecretKeyResolver::Path(
            bob_skey_path.clone(),
            Secret::new_raw(""),
        )]))
        .with_pgp_verify(PgpPublicKey::Enabled(vec![PgpPublicKeyResolver::Raw(
            alice_pkey.clone(),
        )]))
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
