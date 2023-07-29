use concat_with::concat_line;
use pimalaya_email_tpl::{
    PgpPublicKey, PgpPublicKeyResolver, PgpPublicKeys, PgpPublicKeysResolver, PgpSecretKey,
    PgpSecretKeyResolver, Tpl, TplInterpreter,
};
use pimalaya_pgp::generate_key_pair;
use std::collections::HashMap;
use tempfile::tempdir;
use tokio::fs;

// async fn spawn_fake_key_server(pkeys: HashMap<String, String>) -> (String, JoinHandle<()>) {
//     let listener = TcpListener::bind(("localhost", 0)).await.unwrap();
//     let port = listener.local_addr().unwrap().port();
//     let addr = format!("localhost:{port}");
//     println!("addr: {:?}", addr);

//     let handle = task::spawn(async move {
//         loop {
//             let (mut stream, _) = listener.accept().await.unwrap();

//             let mut reader = BufReader::new(&mut stream);
//             let mut email = Vec::new();
//             reader.read_to_end(&mut email).await.unwrap();
//             println!("email: {:?}", String::from_utf8_lossy(&email));
//             stream.write_all(&[]).await.unwrap();

//             // let res = pkeys.get(email.trim()).unwrap();
//             // let res = format!(
//             //     "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
//             //     res.len(),
//             //     res
//             // );
//             // stream.write_all(res.as_bytes()).await.unwrap();
//         }
//     });

//     (addr, handle)
// }

#[tokio::test]
async fn pgp() {
    let dir = tempdir().unwrap();

    let (alice_skey, alice_pkey) = generate_key_pair("alice@localhost").await.unwrap();
    let alice_skey_path = dir.path().join("alice.key");
    fs::write(&alice_skey_path, alice_skey.to_armored_bytes(None).unwrap())
        .await
        .unwrap();

    let (bob_skey, bob_pkey) = generate_key_pair("bob@localhost").await.unwrap();
    let bob_skey_path = dir.path().join("bob.key");
    fs::write(&bob_skey_path, bob_skey.to_armored_bytes(None).unwrap())
        .await
        .unwrap();

    // let (key_server_addr, key_server) = spawn_fake_key_server(HashMap::from_iter([
    //     (
    //         String::from("alice@localhost"),
    //         alice_pkey.to_armored_string(None).unwrap(),
    //     ),
    //     (
    //         String::from("carl@localhost"),
    //         carl_pkey.to_armored_string(None).unwrap(),
    //     ),
    // ]))
    // .await;

    let tpl = Tpl::from(concat_line!(
        "From: alice@localhost",
        "To: bob@localhost",
        "Subject: subject",
        "",
        "<#part type=text/plain encrypt=pgpmime sign=pgpmime>",
        "Encrypted and signed message!",
    ));

    let builder = tpl
        .with_pgp_encrypt(PgpPublicKeys::Enabled(vec![PgpPublicKeysResolver::Raw(
            HashMap::from_iter([
                (String::from("alice@localhost"), alice_pkey.clone()),
                (String::from("bob@localhost"), bob_pkey.clone()),
            ]),
        )]))
        .with_pgp_sign(PgpSecretKey::Enabled(vec![PgpSecretKeyResolver::Path(
            alice_skey_path.clone(),
        )]))
        .compile()
        .await
        .unwrap();

    let tpl = TplInterpreter::new()
        .with_show_only_headers(["From", "To", "Subject"])
        .with_pgp_decrypt(PgpSecretKey::Enabled(vec![PgpSecretKeyResolver::Path(
            bob_skey_path.clone(),
        )]))
        .with_pgp_verify(PgpPublicKey::Enabled(vec![PgpPublicKeyResolver::Raw(
            bob_pkey.clone(),
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

    // key_server.await.unwrap()
}
