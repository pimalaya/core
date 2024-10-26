#[cfg(feature = "async-std")]
use async_std::main;
#[cfg(feature = "tokio")]
use tokio::main;

#[cfg(feature = "pgp-native")]
#[test_log::test(main)]
async fn main() {
    use mml::{
        pgp::{NativePgpPublicKeysResolver, NativePgpSecretKey, Pgp, PgpNative},
        MmlCompilerBuilder,
    };
    use pgp::gen_key_pair;
    use secret::Secret;
    use tempfile::tempdir;
    use tokio::fs;

    let dir = tempdir().unwrap();

    let (alice_skey, _alice_pkey) = gen_key_pair("alice@localhost", "").await.unwrap();
    let alice_skey_path = dir.path().join("alice.key");
    fs::write(&alice_skey_path, alice_skey.to_armored_bytes(None).unwrap())
        .await
        .unwrap();

    let (bob_skey, bob_pkey) = gen_key_pair("bob@localhost", "").await.unwrap();
    let bob_skey_path = dir.path().join("bob.key");
    fs::write(&bob_skey_path, bob_skey.to_armored_bytes(None).unwrap())
        .await
        .unwrap();

    let mml = include_str!("./pgp.eml");
    let mml_compiler = MmlCompilerBuilder::new()
        .with_pgp(Pgp::Native(PgpNative {
            secret_key: NativePgpSecretKey::Path(alice_skey_path.clone()),
            secret_key_passphrase: Secret::new_raw(""),
            public_keys_resolvers: vec![NativePgpPublicKeysResolver::Raw(
                "bob@localhost".into(),
                bob_pkey.clone(),
            )],
        }))
        .build(mml)
        .unwrap();
    let mime = mml_compiler.compile().await.unwrap().into_string().unwrap();

    println!("================================");
    println!("MML MESSAGE");
    println!("================================");
    println!();
    println!("{mml}");

    println!("================================");
    println!("COMPILED MIME MESSAGE");
    println!("================================");
    println!();
    println!("{mime}");
}

#[cfg(not(feature = "pgp-native"))]
#[test_log::test(main)]
async fn main() {
    panic!("The pgp-native cargo feature should be enabled to run this example.");
}
