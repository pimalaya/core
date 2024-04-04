//! Module dedicated to email envelope addresses.
//!
//! This core concept of this module is the [Address] structure, which
//! represents an email envelope address.

use std::hash::{Hash, Hasher};

/// The email envelope address.
///
/// An address is composed of an optional name and
/// an email address.
#[derive(Clone, Debug, Default, Eq, Ord, PartialOrd)]
pub struct Address {
    pub name: Option<String>,
    pub addr: String,
}

impl Hash for Address {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr.hash(state);
    }
}

/// Two addresses are considered equal when their email addresses are
/// equal.
impl PartialEq for Address {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl ToString for Address {
    fn to_string(&self) -> String {
        match &self.name {
            Some(name) => format!("{name} <{}>", self.addr),
            None => self.addr.clone(),
        }
    }
}

impl Address {
    /// Builds a new address from an optional name and an email
    /// address.
    pub fn new(name: Option<impl ToString>, address: impl ToString) -> Self {
        Self {
            name: name.map(|name| name.to_string()),
            addr: address.to_string(),
        }
    }

    /// Builds a new address from an email address only.
    pub fn new_nameless(address: impl ToString) -> Self {
        Self::new(Option::<String>::None, address)
    }
}
