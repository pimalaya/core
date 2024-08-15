//! Module dedicated to Notmuch email envelope flags.
//!
//! This module contains flag-related mapping functions from the
//! [notmuch] crate types.

use notmuch::Message;

use crate::flag::Flags;

use super::Flag;

impl From<&Message> for Flags {
    fn from(msg: &Message) -> Self {
        let mut flags = Flags::default();
        let mut unread = false;

        for tag in msg.tags() {
            match tag.as_str() {
                "draft" => {
                    flags.insert(Flag::Draft);
                }
                "flagged" => {
                    flags.insert(Flag::Flagged);
                }
                "replied" => {
                    flags.insert(Flag::Answered);
                }
                "unread" => {
                    unread = true;
                }
                flag => {
                    flags.insert(Flag::custom(flag));
                }
            }
        }

        if !unread {
            flags.insert(Flag::Seen);
        }

        flags
    }
}
