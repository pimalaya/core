use secrecy::SecretString;

#[derive(Clone, Debug)]
pub enum KeyringEvent {
    SecretRead(SecretString),
    SecretUpdated,
    SecretDeleted,
}
