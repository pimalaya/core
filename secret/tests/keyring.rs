#![cfg(feature = "keyring")]

#[cfg(feature = "async-std")]
use async_std::test;
use secret::{keyring::KeyringEntry, Secret};
#[cfg(feature = "tokio")]
use tokio::test;

#[cfg(feature = "keyring")]
#[test_log::test(test)]
async fn keyring() {
    let entry = KeyringEntry::try_new("key")
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
