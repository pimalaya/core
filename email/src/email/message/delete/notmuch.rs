use async_trait::async_trait;

use crate::{
    envelope::Id, flag::add::notmuch::AddNotmuchFlags,
    message::r#move::notmuch::MoveNotmuchMessages, notmuch::NotmuchContextSync, Result,
};

use super::{default_delete_messages, DeleteMessages};

#[derive(Clone)]
pub struct DeleteNotmuchMessages {
    move_messages: MoveNotmuchMessages,
    add_flags: AddNotmuchFlags,
}

impl DeleteNotmuchMessages {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self {
            move_messages: MoveNotmuchMessages::new(ctx),
            add_flags: AddNotmuchFlags::new(ctx),
        }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn DeleteMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn DeleteMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl DeleteMessages for DeleteNotmuchMessages {
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
