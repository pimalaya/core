use crate::{Flag, Flags};

impl Flags {
    pub fn to_imap_query(&self) -> String {
        self.iter().fold(String::new(), |mut flags, flag| {
            if !flags.is_empty() {
                flags.push(' ')
            }
            flags.push_str(&flag.to_imap_query());
            flags
        })
    }

    pub fn to_imap_flags_vec(&self) -> Vec<imap::types::Flag<'static>> {
        self.iter().map(|flag| flag.clone().into()).collect()
    }
}

impl From<&[imap::types::Flag<'_>]> for Flags {
    fn from(imap_flags: &[imap::types::Flag<'_>]) -> Self {
        Flags::from_iter(imap_flags.iter().flat_map(Flag::try_from))
    }
}

impl From<Vec<imap::types::Flag<'_>>> for Flags {
    fn from(imap_flags: Vec<imap::types::Flag<'_>>) -> Self {
        Flags::from(imap_flags.as_slice())
    }
}

impl Into<Vec<imap::types::Flag<'_>>> for Flags {
    fn into(self) -> Vec<imap::types::Flag<'static>> {
        self.iter()
            .map(ToOwned::to_owned)
            .map(<Flag as Into<imap::types::Flag>>::into)
            .collect()
    }
}
