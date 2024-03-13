#[cfg(target_os = "linux")]
use keyring::KeyutilsEntry;
use keyring::{get_global_service_name, set_global_service_name, KeyringEntry};

#[tokio::test]
async fn test_keyring_entry() {
    env_logger::builder().is_test(true).init();

    // test global keyring
    set_global_service_name("example");
    assert_eq!(get_global_service_name(), "example");
    set_global_service_name("example2");
    assert_eq!(get_global_service_name(), "example");

    // test entry
    let entry = KeyringEntry::try_new("key").unwrap();
    assert_eq!(entry.key, "key");

    // test cache entry
    #[cfg(target_os = "linux")]
    let cache_entry = {
        let cache_entry = KeyutilsEntry::try_new("key").unwrap();
        assert_eq!(entry.key, "key");
        cache_entry
    };

    // test set secret
    entry.set_secret("secret").await.unwrap();
    let secret = entry.get_secret().await.unwrap();
    assert_eq!(secret, "secret");

    #[cfg(target_os = "linux")]
    {
        // test cached secret
        let secret = cache_entry.find_secret().await.unwrap().unwrap();
        assert_eq!(secret, "secret");
    }

    // test delete entry
    entry.delete_secret().await.unwrap();
    assert_eq!(entry.find_secret().await.unwrap(), None);

    #[cfg(target_os = "linux")]
    {
        // test cached secret
        let secret = cache_entry.find_secret().await.unwrap();
        assert_eq!(secret, None);
    }
}
