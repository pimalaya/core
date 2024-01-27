use async_trait::async_trait;

use crate::{
    envelope::Id, flag::add::notmuch::AddNotmuchFlags, message::peek::notmuch::PeekNotmuchMessages,
    notmuch::NotmuchContextSync, Result,
};

use super::{default_get_messages, GetMessages, Messages};

#[derive(Clone)]
pub struct GetNotmuchMessages {
    peek_messages: PeekNotmuchMessages,
    add_flags: AddNotmuchFlags,
}

impl GetNotmuchMessages {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self {
            peek_messages: PeekNotmuchMessages::new(ctx),
            add_flags: AddNotmuchFlags::new(ctx),
        }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn GetMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn GetMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl GetMessages for GetNotmuchMessages {
    async fn get_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        default_get_messages(&self.peek_messages, &self.add_flags, folder, id).await
    }
}
