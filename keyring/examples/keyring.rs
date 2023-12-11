use keyring::Entry;

const KEY: &str = "key";
const VAL: &str = "val";

#[tokio::main]
async fn main() {
    env_logger::builder().is_test(true).init();

    // set global keyring service name
    keyring::set_global_service_name("keyring-example");

    // set entry secret
    let entry = Entry::from(KEY);
    entry.set_secret(VAL).await.unwrap();

    // get entry secret
    let val = entry.get_secret().await.unwrap();
    assert_eq!(entry.get_key(), KEY);
    assert_eq!(val, VAL);

    // delete entry
    entry.delete_secret().await.unwrap();
    assert_eq!(entry.find_secret().await.unwrap(), None);
}
