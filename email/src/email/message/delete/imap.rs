use async_trait::async_trait;

use super::{DefaultDeleteMessages, DeleteMessages};
use crate::{
    account::config::{AccountConfig, HasAccountConfig},
    envelope::Id,
    flag::{
        add::{imap::AddImapFlags, AddFlags},
        Flags,
    },
    imap::ImapContext,
    message::r#move::{imap::MoveImapMessages, MoveMessages},
    AnyResult,
};

#[derive(Clone)]
pub struct DeleteImapMessages {
    move_messages: MoveImapMessages,
    add_flags: AddImapFlags,
}

impl DeleteImapMessages {
    pub fn new(ctx: &ImapContext) -> Self {
        Self {
            move_messages: MoveImapMessages::new(ctx),
            add_flags: AddImapFlags::new(ctx),
        }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn DeleteMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn DeleteMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

impl HasAccountConfig for DeleteImapMessages {
    fn account_config(&self) -> &AccountConfig {
        &self.move_messages.ctx.account_config
    }
}

#[async_trait]
impl MoveMessages for DeleteImapMessages {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        self.move_messages
            .move_messages(from_folder, to_folder, id)
            .await
    }
}

#[async_trait]
impl AddFlags for DeleteImapMessages {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        self.add_flags.add_flags(folder, id, flags).await
    }
}

#[async_trait]
impl DefaultDeleteMessages for DeleteImapMessages {}
