//! Module dedicated to Notmuch email envelope flags.
//!
//! This module contains flag-related mapping functions from the
//! [notmuch] crate types.

use log::debug;
use notmuch::Message;

use crate::flag::Flags;

impl Flags {
    pub fn from_notmuch_msg(msg: &Message) -> Self {
        msg.tags()
            .filter_map(|ref tag| match tag.parse() {
                Ok(flag) => Some(flag),
                Err(err) => {
                    debug!("cannot parse notmuch tag {tag}: {err}");
                    debug!("{err:?}");
                    None
                }
            })
            .collect()
    }
}
