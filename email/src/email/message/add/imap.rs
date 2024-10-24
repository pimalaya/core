use std::borrow::Cow;

use async_trait::async_trait;
use tracing::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::{AddMessage, Flags};
use crate::{envelope::SingleId, imap::ImapContext, AnyResult};

#[derive(Clone, Debug)]
pub struct AddImapMessage {
    ctx: ImapContext,
}

impl AddImapMessage {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn AddMessage> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn AddMessage>> {
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

        let mut client = self.ctx.client().await;
        let config = &client.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let uid = client
            .add_message(
                &folder_encoded,
                flags.to_imap_flags_iter(),
                Cow::Owned(msg.to_vec()),
            )
            .await?;

        Ok(SingleId::from(uid.to_string()))
    }
}
