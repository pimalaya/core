use crate::{debug, info};
use async_trait::async_trait;
use imap_proto::UidSetMember;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::error::Error, envelope::SingleId, imap::ImapContextSync, AnyResult};

use super::{AddMessage, Flags};

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
        raw_msg: &[u8],
        flags: &Flags,
    ) -> AnyResult<SingleId> {
        info!("adding imap message to folder {folder} with flags {flags}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let appended = ctx
            .exec(
                |session| {
                    session
                        .append(&folder, raw_msg)
                        .flags(flags.to_imap_flags_vec())
                        .finish()
                },
                |err| Error::AppendRawMessageWithFlagsImapError(err, folder.clone(), flags.clone()),
            )
            .await?;

        let uid = match appended.uids {
            Some(mut uids) if uids.len() == 1 => match uids.get_mut(0).unwrap() {
                UidSetMember::Uid(uid) => Ok::<_, Error>(*uid),
                UidSetMember::UidRange(uids) => Ok(uids.next().ok_or_else(|| {
                    Error::GetAddedMessageUidFromRangeImapError(uids.fold(
                        String::new(),
                        |range, uid| {
                            if range.is_empty() {
                                uid.to_string()
                            } else {
                                range + ", " + &uid.to_string()
                            }
                        },
                    ))
                })?),
            },
            _ => {
                // TODO: manage other cases
                Err(Error::GetAddedMessageUidImapError)
            }
        }?;
        debug!("added imap message uid: {uid}");

        Ok(SingleId::from(uid.to_string()))
    }
}
