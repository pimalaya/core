/// An address is composed of an optional name and
/// an email address.
#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct Address {
    pub name: Option<String>,
    pub addr: String,
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
    /// Build a new address from an optional name and an email
    /// address.
    pub fn new(name: Option<impl ToString>, address: impl ToString) -> Self {
        Self {
            name: name.map(|name| name.to_string()),
            addr: address.to_string(),
        }
    }

    /// Build a new address from an email address only.
    pub fn new_nameless(address: impl ToString) -> Self {
        Self::new(Option::<String>::None, address)
    }
}
