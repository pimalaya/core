use async_trait::async_trait;
use imap_proto::UidSetMember;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{envelope::SingleId, imap::ImapSessionSync, Result};

use super::AddMessage;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot add  imap message to folder {1}")]
    AppendMessage(#[source] imap::Error, String),
    #[error("cannot get added imap message uid from range {0}")]
    GetAddedMessageUidFromRangeError(String),
    #[error("cannot get added imap message uid: extension UIDPLUS may be missing on the server")]
    GetAddedMessageUidError,
}

#[derive(Clone, Debug)]
pub struct AddMessageImap {
    session: ImapSessionSync,
}

impl AddMessageImap {
    pub fn new(session: &ImapSessionSync) -> Option<Box<dyn AddMessage>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl AddMessage for AddMessageImap {
    async fn add_message(&self, folder: &str, raw_msg: &[u8]) -> Result<SingleId> {
        info!("adding imap message to folder {folder}");

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let appended = session
            .execute(
                |session| session.append(&folder, raw_msg).finish(),
                |err| Error::AppendMessage(err, folder.clone()).into(),
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
