#![cfg(feature = "command")]

#[cfg(feature = "async-std")]
use async_std::test;
use secret::Secret;
#[cfg(feature = "tokio")]
use tokio::test;

#[test_log::test(test)]
async fn test_command() {
    let mut secret = Secret::new_command("echo 'secret'");
    assert_eq!(secret.get().await.unwrap(), "secret");

    secret.set("secret2").await.unwrap();
    // secret cannot be changed from command variant
    assert_eq!(secret.get().await.unwrap(), "secret");

    secret.delete().await.unwrap();
    assert_eq!(secret.find().await.unwrap(), None);
}
