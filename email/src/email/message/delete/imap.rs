use crate::{
    account::{AccountConfig, WithAccountConfig},
    imap::ImapSessionSync,
};

#[derive(Clone, Debug)]
pub struct DeleteImapMessages {
    session: ImapSessionSync,
}

impl WithAccountConfig for DeleteImapMessages {
    fn account_config(&self) -> &AccountConfig {
        &self.session.account_config
    }
}
