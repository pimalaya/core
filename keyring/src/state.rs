use secrecy::SecretString;

#[derive(Clone, Debug)]
pub enum KeyringState {
    ReadSecret,
    UpdateSecret(SecretString),
    DeleteSecret,
}

#[derive(Debug, Clone)]
pub struct KeyringState2 {
    pub service: String,
    pub account: String,
    pub state: Option<KeyringState>,
}

impl KeyringState2 {
    pub fn new(service: impl ToString, account: impl ToString) -> Self {
        Self {
            service: service.to_string(),
            account: account.to_string(),
            state: None,
        }
    }

    pub fn read_secret(&mut self) {
        self.state = Some(KeyringState::ReadSecret);
    }

    pub fn update_secret(&mut self, secret: impl Into<SecretString>) {
        self.state = Some(KeyringState::UpdateSecret(secret.into()));
    }

    pub fn delete_secret(&mut self) {
        self.state = Some(KeyringState::DeleteSecret);
    }
}

impl Iterator for KeyringState2 {
    type Item = KeyringState;

    fn next(&mut self) -> Option<Self::Item> {
        self.state.take()
    }
}
