use crate::{Flag, Flags};

impl From<&maildir::MailEntry> for Flags {
    fn from(entry: &maildir::MailEntry) -> Self {
        entry.flags().chars().flat_map(Flag::try_from).collect()
    }
}

impl Flags {
    pub fn to_normalized_string(&self) -> String {
        String::from_iter(self.iter().filter_map(<&Flag as Into<Option<char>>>::into))
    }
}
