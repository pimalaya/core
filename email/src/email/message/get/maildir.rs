use async_trait::async_trait;

use crate::{
    envelope::Id, flag::add::maildir::AddMaildirFlags, maildir::MaildirContextSync,
    message::peek::maildir::PeekMaildirMessages, Result,
};

use super::{default_get_messages, GetMessages, Messages};

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
impl GetMessages for GetMaildirMessages {
    async fn get_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        default_get_messages(&self.peek_messages, &self.add_flags, folder, id).await
    }
}
