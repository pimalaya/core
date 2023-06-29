//! Module dedicated to Maildir email envelope flags.
//!
//! This module contains flag-related mapping functions from the
//! [maildirpp] crate types.

use log::{debug, warn};
use maildirpp::MailEntry;
use thiserror::Error;

use crate::Result;

use super::{Flag, Flags};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse maildir flag char {0}")]
    ParseFlagError(char),
}

impl Flag {
    pub fn try_from_mdir_char(c: char) -> Result<Self> {
        match c {
            'r' | 'R' => Ok(Flag::Answered),
            's' | 'S' => Ok(Flag::Seen),
            't' | 'T' => Ok(Flag::Deleted),
            'd' | 'D' => Ok(Flag::Draft),
            'f' | 'F' => Ok(Flag::Flagged),
            unknown => Ok(Err(Error::ParseFlagError(unknown))?),
        }
    }

    pub fn to_opt_mdir_char(&self) -> Option<char> {
        match self {
            Flag::Answered => Some('R'),
            Flag::Seen => Some('S'),
            Flag::Deleted => Some('T'),
            Flag::Draft => Some('D'),
            Flag::Flagged => Some('F'),
            _ => None,
        }
    }
}

impl Flags {
    pub fn from_mdir_entry(entry: &MailEntry) -> Self {
        entry
            .flags()
            .chars()
            .filter_map(|c| match Flag::try_from_mdir_char(c) {
                Ok(flag) => Some(flag),
                Err(err) => {
                    warn!("cannot parse maildir flag char {c}, skipping it: {err}");
                    debug!("cannot parse maildir flag char {c}: {err:?}");
                    None
                }
            })
            .collect()
    }

    pub fn to_mdir_string(&self) -> String {
        String::from_iter(self.iter().filter_map(|flag| flag.to_opt_mdir_char()))
    }
}
