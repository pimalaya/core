use async_trait::async_trait;

use super::{DefaultGetMessages, GetMessages, Messages};
use crate::{
    envelope::Id,
    flag::{
        add::{maildir::AddMaildirFlags, AddFlags},
        Flags,
    },
    maildir::MaildirContextSync,
    message::peek::{maildir::PeekMaildirMessages, PeekMessages},
    AnyResult,
};

#[derive(Clone)]
pub struct GetMaildirMessages {
    peek_messages: PeekMaildirMessages,
    add_flags: AddMaildirFlags,
}

impl GetMaildirMessages {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self {
            peek_messages: PeekMaildirMessages::new(ctx),
            add_flags: AddMaildirFlags::new(ctx),
        }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn GetMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn GetMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl PeekMessages for GetMaildirMessages {
    async fn peek_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        self.peek_messages.peek_messages(folder, id).await
    }
}

#[async_trait]
impl AddFlags for GetMaildirMessages {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        self.add_flags.add_flags(folder, id, flags).await
    }
}

#[async_trait]
impl DefaultGetMessages for GetMaildirMessages {}
