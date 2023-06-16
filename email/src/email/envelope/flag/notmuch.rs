use log::{debug, warn};
use notmuch::Message;

use crate::Flags;

impl Flags {
    pub fn from_notmuch_msg(msg: &Message) -> Self {
        msg.tags()
            .filter_map(|ref tag| match tag.parse() {
                Ok(flag) => Some(flag),
                Err(err) => {
                    warn!("cannot parse notmuch tag {tag}, skipping it: {err}");
                    debug!("cannot parse notmuch tag {tag}: {err:?}");
                    None
                }
            })
            .collect()
    }
}
