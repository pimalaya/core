use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    ops,
    str::FromStr,
};

use crate::Flag;

use super::{Error, Result};

/// Represents the list of flags.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Flags(pub HashSet<Flag>);

impl Hash for Flags {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for flag in self.iter() {
            flag.hash(state)
        }
    }
}

impl ToString for Flags {
    fn to_string(&self) -> String {
        self.iter().fold(String::new(), |mut flags, flag| {
            if !flags.is_empty() {
                flags.push(' ')
            }
            flags.push_str(&flag.to_string());
            flags
        })
    }
}

impl ops::Deref for Flags {
    type Target = HashSet<Flag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Flags {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<&str> for Flags {
    fn from(s: &str) -> Self {
        s.split_whitespace().flat_map(|flag| flag.parse()).collect()
    }
}

impl From<String> for Flags {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl FromStr for Flags {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(Flags(
            s.split_whitespace()
                .map(|flag| flag.parse())
                .collect::<Result<HashSet<_>>>()?,
        ))
    }
}

impl FromIterator<Flag> for Flags {
    fn from_iter<T: IntoIterator<Item = Flag>>(iter: T) -> Self {
        let mut flags = Flags::default();
        flags.extend(iter);
        flags
    }
}

impl Into<Vec<String>> for Flags {
    fn into(self) -> Vec<String> {
        self.iter().map(|flag| flag.to_string()).collect()
    }
}
