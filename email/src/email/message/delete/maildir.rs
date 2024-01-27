use async_trait::async_trait;

use crate::{
    envelope::Id, flag::add::maildir::AddMaildirFlags, maildir::MaildirContextSync,
    message::r#move::maildir::MoveMaildirMessages, Result,
};

use super::{default_delete_messages, DeleteMessages};

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

#[async_trait]
impl DeleteMessages for DeleteMaildirMessages {
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
