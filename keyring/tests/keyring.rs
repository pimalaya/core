use keyring::{set_global_service_name, Entry};

const KEY: &str = "key";
const VAL: &str = "val";

#[tokio::test]
async fn test_keyring_entry() {
    env_logger::builder().is_test(true).init();

    // set global keyring service name
    set_global_service_name("example");

    // set entry secret
    let entry = Entry::new(KEY);
    entry.set_secret(VAL).await.unwrap();

    // get entry secret
    let val = entry.get_secret().await.unwrap();
    assert_eq!(entry.get_key(), KEY);
    assert_eq!(val, VAL);

    // delete entry
    entry.delete_secret().await.unwrap();
    assert_eq!(entry.find_secret().await.unwrap(), None);
}
