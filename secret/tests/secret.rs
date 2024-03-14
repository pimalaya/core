use secret::{keyring, Secret};

#[tokio::test]
async fn test_secret_variants() {
    env_logger::builder().is_test(true).init();

    // test raw secret

    let mut secret = Secret::new_raw("secret");
    assert_eq!(secret.get().await.unwrap(), "secret");

    secret.set("secret2").await.unwrap();
    assert_eq!(secret.get().await.unwrap(), "secret2");

    secret.delete().await.unwrap();
    assert_eq!(secret.find().await.unwrap(), None);

    // test cmd secret

    let mut secret = Secret::new_command("echo 'secret'");
    assert_eq!(secret.get().await.unwrap(), "secret");

    secret.set("secret2").await.unwrap();
    // secret cannot be changed from command variant
    assert_eq!(secret.get().await.unwrap(), "secret");

    secret.delete().await.unwrap();
    assert_eq!(secret.find().await.unwrap(), None);

    // test keyring secret

    let entry = keyring::KeyringEntry::try_new("key")
        .unwrap()
        .try_with_secret("secret")
        .await
        .unwrap();
    let mut secret = Secret::new_keyring_entry(entry);
    assert_eq!(secret.get().await.unwrap(), "secret");

    secret.set("secret2").await.unwrap();
    assert_eq!(secret.get().await.unwrap(), "secret2");

    secret.delete().await.unwrap();
    assert_eq!(secret.find().await.unwrap(), None);
}
