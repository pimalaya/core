use async_trait::async_trait;

use super::{DefaultDeleteMessages, DeleteMessages};
use crate::{
    account::config::{AccountConfig, HasAccountConfig},
    envelope::Id,
    flag::{
        add::{notmuch::AddNotmuchFlags, AddFlags},
        Flags,
    },
    message::r#move::{notmuch::MoveNotmuchMessages, MoveMessages},
    notmuch::NotmuchContextSync,
    AnyResult,
};

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

impl HasAccountConfig for DeleteNotmuchMessages {
    fn account_config(&self) -> &AccountConfig {
        &self.move_messages.ctx.account_config
    }
}

#[async_trait]
impl MoveMessages for DeleteNotmuchMessages {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        self.move_messages
            .move_messages(from_folder, to_folder, id)
            .await
    }
}

#[async_trait]
impl AddFlags for DeleteNotmuchMessages {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        self.add_flags.add_flags(folder, id, flags).await
    }
}

#[async_trait]
impl DefaultDeleteMessages for DeleteNotmuchMessages {}
