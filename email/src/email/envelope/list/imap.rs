use async_trait::async_trait;
use log::{debug, info};
use std::result;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSessionSync, Result};

use super::{Envelopes, ListEnvelopes};

/// The IMAP query needed to retrieve everything we need to build an
/// [envelope]: UID, flags and headers (Message-ID, From, To, Subject,
/// Date).
const LIST_ENVELOPES_QUERY: &str =
    "(UID FLAGS BODY.PEEK[HEADER.FIELDS (MESSAGE-ID FROM TO SUBJECT DATE)])";

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot list imap envelopes {2} from folder {1}")]
    ListEnvelopesError(#[source] imap::Error, String, String),
    #[error("cannot list imap envelopes: page {0} out of bounds")]
    BuildPageRangeOutOfBoundsError(usize),
}

#[derive(Clone, Debug)]
pub struct ListEnvelopesImap {
    session: ImapSessionSync,
}

impl ListEnvelopesImap {
    pub fn new(session: &ImapSessionSync) -> Option<Box<dyn ListEnvelopes>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl ListEnvelopes for ListEnvelopesImap {
    async fn list_envelopes(
        &self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        info!("listing imap envelopes from folder {folder}");

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let folder_size = session
            .execute(
                |session| session.select(&folder_encoded),
                |err| Error::SelectFolderError(err, folder.clone()).into(),
            )
            .await?
            .exists as usize;
        debug!("folder size: {folder_size}");

        if folder_size == 0 {
            return Ok(Envelopes::default());
        }

        let range = build_page_range(page, page_size, folder_size)?;
        debug!("page range: {range}");

        let fetches = session
            .execute(
                |session| session.fetch(&range, LIST_ENVELOPES_QUERY),
                |err| Error::ListEnvelopesError(err, folder.clone(), range.clone()).into(),
            )
            .await?;

        let envelopes = Envelopes::from_imap_fetches(fetches);
        debug!("imap envelopes: {envelopes:#?}");

        Ok(envelopes)
    }
}

/// Builds the IMAP sequence set for the give page, page size and
/// total size.
fn build_page_range(page: usize, page_size: usize, size: usize) -> result::Result<String, Error> {
    let page_cursor = page * page_size;
    if page_cursor >= size {
        return Err(Error::BuildPageRangeOutOfBoundsError(page + 1))?;
    }

    let range = if page_size == 0 {
        String::from("1:*")
    } else {
        let page_size = page_size.min(size);
        let mut count = 1;
        let mut cursor = size - (size.min(page_cursor));
        let mut range = cursor.to_string();
        while cursor > 1 && count < page_size {
            count += 1;
            cursor -= 1;
            if count > 1 {
                range.push(',');
            }
            range.push_str(&cursor.to_string());
        }
        range
    };

    Ok(range)
}

#[cfg(test)]
mod tests {
    #[test]
    fn build_page_range_out_of_bounds() {
        // page * page_size < size
        assert_eq!(super::build_page_range(0, 5, 5).unwrap(), "5,4,3,2,1");

        // page * page_size = size
        assert!(matches!(
            super::build_page_range(1, 5, 5).unwrap_err(),
            super::Error::BuildPageRangeOutOfBoundsError(2),
        ));

        // page * page_size > size
        assert!(matches!(
            super::build_page_range(2, 5, 5).unwrap_err(),
            super::Error::BuildPageRangeOutOfBoundsError(3),
        ));
    }

    #[test]
    fn build_page_range_page_size_0() {
        assert_eq!(super::build_page_range(0, 0, 3).unwrap(), "1:*");
        assert_eq!(super::build_page_range(1, 0, 4).unwrap(), "1:*");
        assert_eq!(super::build_page_range(2, 0, 5).unwrap(), "1:*");
    }

    #[test]
    fn build_page_range_page_size_smaller_than_size() {
        assert_eq!(super::build_page_range(0, 3, 5).unwrap(), "5,4,3");
        assert_eq!(super::build_page_range(1, 3, 5).unwrap(), "2,1");
        assert_eq!(super::build_page_range(1, 4, 5).unwrap(), "1");
    }

    #[test]
    fn build_page_range_page_bigger_than_size() {
        assert_eq!(super::build_page_range(0, 10, 5).unwrap(), "5,4,3,2,1");
    }
}
