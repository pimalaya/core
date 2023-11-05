use async_trait::async_trait;
use imap_proto::UidSetMember;
use log::{debug, info};
use std::error;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSessionSync, Result};

use super::{AddEmail, Flags};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot add raw email to the imap folder {1}")]
    AppendRawEmailError(#[source] imap::Error, String),
    #[error("cannot get added email imap uid from range {0}")]
    GetAddedEmailUidFromRangeError(String),
    #[error("cannot get added email imap uid (extensions UIDPLUS not enabled on the server?)")]
    GetAddedEmailUidError,
}

impl Error {
    pub fn append_raw_email(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::AppendRawEmailError(err, folder))
    }
}

#[derive(Clone, Debug)]
pub struct AddImapEmail {
    session: ImapSessionSync,
}

impl AddImapEmail {
    pub fn new(session: &ImapSessionSync) -> Box<dyn AddEmail> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl AddEmail for AddImapEmail {
    async fn add_email(&self, folder: &str, email: &[u8], flags: &Flags) -> Result<String> {
        info!(
            "adding imap email to folder {folder} with flags {flags}",
            flags = flags.to_string(),
        );

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let appended = session
            .execute(
                |session| {
                    session
                        .append(&folder, email)
                        .flags(flags.to_imap_flags_vec())
                        .finish()
                },
                |err| Error::append_raw_email(err, folder.clone()),
            )
            .await?;

        let uid = match appended.uids {
            Some(mut uids) if uids.len() == 1 => match uids.get_mut(0).unwrap() {
                UidSetMember::Uid(uid) => Ok(*uid),
                UidSetMember::UidRange(uids) => Ok(uids.next().ok_or_else(|| {
                    crate::imap::Error::ExecuteSessionActionError(Box::new(
                        Error::GetAddedEmailUidFromRangeError(uids.fold(
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
                // TODO: find a way to retrieve the UID of the added
                // email (by Message-ID?)
                Err(crate::imap::Error::ExecuteSessionActionError(Box::new(
                    Error::GetAddedEmailUidError,
                )))
            }
        }?;
        debug!("uid: {uid}");

        Ok(uid.to_string())
    }
}
