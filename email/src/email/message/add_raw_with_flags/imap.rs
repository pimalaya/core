use async_trait::async_trait;
use imap_proto::UidSetMember;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::envelope::SingleId, imap::ImapSessionSync, Result};

use super::{AddRawMessageWithFlags, Flags};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot add raw imap message to folder {1} with flags {2}")]
    AppendRawMessageWithFlagsError(#[source] imap::Error, String, Flags),
    #[error("cannot get added imap message uid from range {0}")]
    GetAddedMessageUidFromRangeError(String),
    #[error("cannot get added imap message uid: extension UIDPLUS may be missing on the server")]
    GetAddedMessageUidError,
}

#[derive(Clone, Debug)]
pub struct AddRawMessageWithFlagsImap {
    session: ImapSessionSync,
}

impl AddRawMessageWithFlagsImap {
    pub fn new(session: &ImapSessionSync) -> Option<Box<dyn AddRawMessageWithFlags>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl AddRawMessageWithFlags for AddRawMessageWithFlagsImap {
    async fn add_raw_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        info!("adding imap message to folder {folder} with flags {flags}");

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let appended = session
            .execute(
                |session| {
                    session
                        .append(&folder, raw_msg)
                        .flags(flags.to_imap_flags_vec())
                        .finish()
                },
                |err| {
                    Error::AppendRawMessageWithFlagsError(err, folder.clone(), flags.clone()).into()
                },
            )
            .await?;

        let uid = match appended.uids {
            Some(mut uids) if uids.len() == 1 => match uids.get_mut(0).unwrap() {
                UidSetMember::Uid(uid) => anyhow::Ok(*uid),
                UidSetMember::UidRange(uids) => Ok(uids.next().ok_or_else(|| {
                    Error::GetAddedMessageUidFromRangeError(uids.fold(
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
                Err(Error::GetAddedMessageUidError.into())
            }
        }?;
        debug!("added imap message uid: {uid}");

        Ok(SingleId::from(uid.to_string()))
    }
}
