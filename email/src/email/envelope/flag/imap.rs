use imap::{self, types::Fetch};
use log::{debug, warn};
use std::result;
use thiserror::Error;

use crate::{Flag, Flags};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse imap flag {0}")]
    ParseFlagError(String),
}

type Result<T> = result::Result<T, Error>;

impl Flag {
    pub fn try_from_imap_flag(imap_flag: &imap::types::Flag) -> Result<Self> {
        match imap_flag {
            imap::types::Flag::Seen => Ok(Flag::Seen),
            imap::types::Flag::Answered => Ok(Flag::Answered),
            imap::types::Flag::Flagged => Ok(Flag::Flagged),
            imap::types::Flag::Deleted => Ok(Flag::Deleted),
            imap::types::Flag::Draft => Ok(Flag::Draft),
            unknown => Err(Error::ParseFlagError(unknown.to_string())),
        }
    }

    pub fn to_imap_query_string(&self) -> String {
        match self {
            Flag::Seen => String::from("\\Seen"),
            Flag::Answered => String::from("\\Answered"),
            Flag::Flagged => String::from("\\Flagged"),
            Flag::Deleted => String::from("\\Deleted"),
            Flag::Draft => String::from("\\Draft"),
            Flag::Custom(flag) => flag.clone(),
        }
    }

    pub fn to_imap_flag(&self) -> imap::types::Flag<'static> {
        match self {
            Flag::Seen => imap::types::Flag::Seen,
            Flag::Answered => imap::types::Flag::Answered,
            Flag::Flagged => imap::types::Flag::Flagged,
            Flag::Deleted => imap::types::Flag::Deleted,
            Flag::Draft => imap::types::Flag::Draft,
            Flag::Custom(flag) => imap::types::Flag::Custom(flag.to_owned().into()),
        }
    }
}

impl Flags {
    pub fn from_imap_fetch(fetch: &Fetch) -> Self {
        Flags::from_iter(fetch.flags().iter().filter_map(|flag| {
            match Flag::try_from_imap_flag(flag) {
                Ok(flag) => Some(flag),
                Err(err) => {
                    warn!("cannot parse imap flag {flag}, skipping it: {err}");
                    debug!("cannot parse imap flag {flag}: {err:?}");
                    None
                }
            }
        }))
    }

    pub fn to_imap_query_string(&self) -> String {
        self.iter().fold(String::new(), |mut flags, flag| {
            if !flags.is_empty() {
                flags.push(' ')
            }
            flags.push_str(&flag.to_imap_query_string());
            flags
        })
    }

    pub fn to_imap_flags_vec(&self) -> Vec<imap::types::Flag<'_>> {
        self.iter().map(|flag| flag.to_imap_flag()).collect()
    }
}