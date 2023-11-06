use async_trait::async_trait;
use imap_proto::UidSetMember;
use log::{debug, info};
use std::error;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::envelope::Id, imap::ImapSessionSync, Result};

use super::AddRawMessage;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot add raw imap message to folder {1}")]
    AppendRawMessage(#[source] imap::Error, String),
    #[error("cannot get added imap message uid from range {0}")]
    GetAddedMessageUidFromRangeError(String),
    #[error("cannot get added imap message uid: extension UIDPLUS may be missing on the server")]
    GetAddedMessageUidError,
}

impl Error {
    pub fn append_raw_message(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::AppendRawMessage(err, folder))
    }
}

#[derive(Clone, Debug)]
pub struct AddRawImapMessage {
    session: ImapSessionSync,
}

impl AddRawImapMessage {
    pub fn new(session: &ImapSessionSync) -> Box<dyn AddRawMessage> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl AddRawMessage for AddRawImapMessage {
    async fn add_raw_message(&self, folder: &str, raw_msg: &[u8]) -> Result<Id> {
        info!("adding imap message to folder {folder}");

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let appended = session
            .execute(
                |session| session.append(&folder, raw_msg).finish(),
                |err| Error::append_raw_message(err, folder.clone()),
            )
            .await?;

        let uid = match appended.uids {
            Some(mut uids) if uids.len() == 1 => match uids.get_mut(0).unwrap() {
                UidSetMember::Uid(uid) => Ok(*uid),
                UidSetMember::UidRange(uids) => Ok(uids.next().ok_or_else(|| {
                    crate::imap::Error::ExecuteSessionActionError(Box::new(
                        Error::GetAddedMessageUidFromRangeError(uids.fold(
                            String::new(),
                            |range, uid| {
                                if range.is_empty() {
                                    uid.to_string()
                                } else {
                                    range + ", " + &uid.to_string()
                                }
                            },
                        )),
                    ))
                })?),
            },
            _ => {
                // TODO: manage other cases
                Err(crate::imap::Error::ExecuteSessionActionError(Box::new(
                    Error::GetAddedMessageUidError,
                )))
            }
        }?;
        debug!("added imap message uid: {uid}");

        Ok(Id::single(uid))
    }
}
