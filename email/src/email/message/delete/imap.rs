use async_trait::async_trait;

use crate::{
    envelope::Id, flag::add::imap::AddImapFlags, imap::ImapContextSync,
    message::r#move::imap::MoveImapMessages, Result,
};

use super::{default_delete_messages, DeleteMessages};

#[derive(Clone)]
pub struct DeleteImapMessages {
    move_messages: MoveImapMessages,
    add_flags: AddImapFlags,
}

impl DeleteImapMessages {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self {
            move_messages: MoveImapMessages::new(ctx),
            add_flags: AddImapFlags::new(ctx),
        }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn DeleteMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn DeleteMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl DeleteMessages for DeleteImapMessages {
    async fn delete_messages(&self, folder: &str, id: &Id) -> Result<()> {
        default_delete_messages(
            &self.move_messages.ctx.account_config,
            &self.move_messages,
            &self.add_flags,
            folder,
            id,
        )
        .await
    }
}
