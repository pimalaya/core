use async_trait::async_trait;

use crate::{
    envelope::Id,
    flag::{
        add::{notmuch::AddNotmuchFlags, AddFlags},
        Flags,
    },
    message::peek::{notmuch::PeekNotmuchMessages, PeekMessages},
    notmuch::NotmuchContextSync,
    Result,
};

use super::{DefaultGetMessages, GetMessages, Messages};

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
impl PeekMessages for GetNotmuchMessages {
    async fn peek_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        self.peek_messages.peek_messages(folder, id).await
    }
}

#[async_trait]
impl AddFlags for GetNotmuchMessages {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.add_flags.add_flags(folder, id, flags).await
    }
}

#[async_trait]
impl DefaultGetMessages for GetNotmuchMessages {}
