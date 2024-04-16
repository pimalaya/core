//! Module dedicated to email utils.

use crate::debug;
use std::{env, fs, io, path::PathBuf};

/// Gets the local draft file path.
pub fn local_draft_path() -> PathBuf {
    let path = env::temp_dir().join("himalaya-draft.eml");
    debug!("local draft path: {}", path.display());
    path
}

/// Removes the local draft.
pub fn remove_local_draft() -> io::Result<()> {
    let path = local_draft_path();
    fs::remove_file(path)?;
    Ok(())
}

/// Module dedicated to email address utils.
pub(crate) mod address {
    use mail_builder::headers::address::Address as AddressBuilder;
    use mail_parser::{Address, HeaderValue};
    use std::borrow::Cow;

    pub(crate) fn is_empty(header: &HeaderValue) -> bool {
        match header {
            HeaderValue::Address(Address::List(addrs)) => addrs.is_empty(),
            HeaderValue::Address(Address::Group(groups)) => groups.is_empty(),
            HeaderValue::Empty => true,
            _ => false,
        }
    }

    pub(crate) fn contains(header: &HeaderValue, a: &Option<Cow<str>>) -> bool {
        match header {
            HeaderValue::Address(Address::List(addrs)) => addrs.iter().any(|b| a == &b.address),
            HeaderValue::Address(Address::Group(groups)) => groups
                .iter()
                .find_map(|g| g.addresses.iter().find(|b| a == &b.address))
                .is_some(),
            _ => false,
        }
    }

    pub(crate) fn get_address_id(header: &HeaderValue) -> Vec<String> {
        match header {
            HeaderValue::Address(Address::List(addrs)) => addrs
                .iter()
                .map(|a| a.address.clone().unwrap_or_default().to_string())
                .collect(),
            HeaderValue::Address(Address::Group(groups)) => groups
                .iter()
                .map(|g| g.name.clone().unwrap_or_default().to_string())
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn into(header: HeaderValue) -> AddressBuilder {
        match header {
            HeaderValue::Address(Address::List(addrs)) => AddressBuilder::new_list(
                addrs
                    .into_iter()
                    .filter_map(|a| {
                        a.address
                            .map(|email| AddressBuilder::new_address(a.name, email))
                    })
                    .collect(),
            ),
            HeaderValue::Address(Address::Group(groups)) => AddressBuilder::new_list(
                groups
                    .into_iter()
                    .flat_map(|g| {
                        g.addresses.into_iter().filter_map(|a| {
                            a.address
                                .map(|email| AddressBuilder::new_address(a.name, email))
                        })
                    })
                    .collect(),
            ),
            _ => AddressBuilder::new_list(Vec::new()),
        }
    }

    pub(crate) fn equal(a: &HeaderValue, b: &HeaderValue) -> bool {
        get_address_id(a) == get_address_id(b)
    }
}
