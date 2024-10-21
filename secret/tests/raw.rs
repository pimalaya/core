#[cfg(feature = "async-std")]
use async_std::test;
use secret::Secret;
#[cfg(feature = "tokio")]
use tokio::test;

#[test_log::test(test)]
async fn raw() {
    let mut secret = Secret::new_raw("secret");
    assert_eq!(secret.get().await.unwrap(), "secret");

    secret.set("secret2").await.unwrap();
    assert_eq!(secret.get().await.unwrap(), "secret2");

    secret.delete().await.unwrap();
    assert_eq!(secret.find().await.unwrap(), None);
}
