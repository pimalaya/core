use async_trait::async_trait;
use log::{debug, info};
use std::error;
use thiserror::Error;

use crate::{maildir::MaildirSessionSync, Result};

use super::{Envelopes, ListEnvelopes};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot list maildir envelopes from {0}: page {1} out of bounds")]
    GetEnvelopesOutOfBoundsError(String, usize),
}

impl Error {
    pub fn out_of_bounds(folder: &str, page: usize) -> Box<dyn error::Error + Send> {
        Box::new(Self::GetEnvelopesOutOfBoundsError(folder.to_owned(), page))
    }
}

#[derive(Clone)]
pub struct ListEnvelopesMaildir {
    session: MaildirSessionSync,
}

impl ListEnvelopesMaildir {
    pub fn new(session: &MaildirSessionSync) -> Box<dyn ListEnvelopes> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl ListEnvelopes for ListEnvelopesMaildir {
    async fn list_envelopes(
        &self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        info!("listing envelopes from maildir folder {folder}");

        let session = self.session.lock().await;
        let mdir = session.get_mdir_from_dir(folder)?;

        let mut envelopes = Envelopes::from_mdir_entries(mdir.list_cur());
        debug!("maildir envelopes: {envelopes:#?}");

        let page_begin = page * page_size;
        debug!("page begin: {}", page_begin);
        if page_begin > envelopes.len() {
            return Err(Error::out_of_bounds(folder, page_begin + 1))?;
        }

        let page_end = envelopes.len().min(if page_size == 0 {
            envelopes.len()
        } else {
            page_begin + page_size
        });
        debug!("page end: {}", page_end);

        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());
        *envelopes = envelopes[page_begin..page_end].into();

        Ok(envelopes)
    }
}
