use async_trait::async_trait;

use super::{DefaultDeleteMessages, DeleteMessages};
use crate::{
    account::config::{AccountConfig, HasAccountConfig},
    envelope::Id,
    flag::{
        add::{maildir::AddMaildirFlags, AddFlags},
        Flags,
    },
    maildir::MaildirContextSync,
    message::r#move::{maildir::MoveMaildirMessages, MoveMessages},
    AnyResult,
};

#[derive(Clone)]
pub struct DeleteMaildirMessages {
    move_messages: MoveMaildirMessages,
    add_flags: AddMaildirFlags,
}

impl DeleteMaildirMessages {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self {
            move_messages: MoveMaildirMessages::new(ctx),
            add_flags: AddMaildirFlags::new(ctx),
        }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn DeleteMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn DeleteMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

impl HasAccountConfig for DeleteMaildirMessages {
    fn account_config(&self) -> &AccountConfig {
        &self.move_messages.ctx.account_config
    }
}

#[async_trait]
impl MoveMessages for DeleteMaildirMessages {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        self.move_messages
            .move_messages(from_folder, to_folder, id)
            .await
    }
}

#[async_trait]
impl AddFlags for DeleteMaildirMessages {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        self.add_flags.add_flags(folder, id, flags).await
    }
}

#[async_trait]
impl DefaultDeleteMessages for DeleteMaildirMessages {}
