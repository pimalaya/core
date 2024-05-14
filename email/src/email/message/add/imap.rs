use std::borrow::Cow;

use async_trait::async_trait;
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::{AddMessage, Flags};
use crate::{debug, envelope::SingleId, imap::ImapContextSync, info, AnyResult};

#[derive(Clone, Debug)]
pub struct AddImapMessage {
    ctx: ImapContextSync,
}

impl AddImapMessage {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn AddMessage> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn AddMessage>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl AddMessage for AddImapMessage {
    async fn add_message_with_flags(
        &self,
        folder: &str,
        msg: &[u8],
        flags: &Flags,
    ) -> AnyResult<SingleId> {
        info!("adding imap message to folder {folder} with flags {flags}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let uid = ctx
            .add_message(
                &folder_encoded,
                flags.to_imap_flags_iter(),
                Cow::Owned(msg.to_vec()),
            )
            .await?;

        Ok(SingleId::from(uid.to_string()))
    }
}
