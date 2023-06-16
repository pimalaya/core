//! Module dedicated to email utils.

use log::debug;
use std::{env, fs, io, path::PathBuf};

/// Get the local draft file path.
pub fn local_draft_path() -> PathBuf {
    let path = env::temp_dir().join("himalaya-draft.eml");
    debug!("local draft path: {}", path.display());
    path
}

/// Remove the local draft.
pub fn remove_local_draft() -> io::Result<()> {
    let path = local_draft_path();
    fs::remove_file(&path)?;
    Ok(())
}

/// Module dedicated to email address utils.
pub(crate) mod address {
    use mail_builder::headers::address::Address;
    use mail_parser::HeaderValue;
    use std::borrow::Cow;

    pub(crate) fn is_empty(header: &HeaderValue) -> bool {
        match header {
            HeaderValue::AddressList(addresses) => addresses.is_empty(),
            HeaderValue::Group(group) => group.addresses.is_empty(),
            HeaderValue::GroupList(groups) => groups.is_empty() || groups[0].addresses.is_empty(),
            HeaderValue::Empty => true,
            _ => false,
        }
    }

    pub(crate) fn contains(header: &HeaderValue, a: &Option<Cow<str>>) -> bool {
        match header {
            HeaderValue::Address(b) => a == &b.address,
            HeaderValue::AddressList(addresses) => {
                addresses.iter().find(|b| a == &b.address).is_some()
            }
            HeaderValue::Group(group) => group.addresses.iter().find(|b| a == &b.address).is_some(),
            HeaderValue::GroupList(groups) => groups
                .iter()
                .find(|group| group.addresses.iter().find(|b| a == &b.address).is_some())
                .is_some(),
            _ => false,
        }
    }

    pub(crate) fn get_address_id(header: &HeaderValue) -> Vec<String> {
        match header {
            HeaderValue::Address(a) => {
                vec![a.address.clone().unwrap_or_default().to_string()]
            }
            HeaderValue::AddressList(addresses) => addresses
                .iter()
                .map(|a| a.address.clone().unwrap_or_default().to_string())
                .collect(),
            HeaderValue::Group(group) => vec![group.name.clone().unwrap_or_default().to_string()],
            HeaderValue::GroupList(groups) => groups
                .iter()
                .map(|group| group.name.clone().unwrap_or_default().to_string())
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(crate) fn into(header: HeaderValue) -> Address {
        match header {
            HeaderValue::Address(a) if a.address.is_some() => {
                Address::new_address(a.name, a.address.unwrap())
            }
            HeaderValue::AddressList(a) => Address::new_list(
                a.into_iter()
                    .filter_map(|a| a.address.map(|email| Address::new_address(a.name, email)))
                    .collect(),
            ),
            HeaderValue::Group(g) => Address::new_group(
                g.name,
                g.addresses
                    .into_iter()
                    .filter_map(|a| a.address.map(|email| Address::new_address(a.name, email)))
                    .collect(),
            ),
            _ => Address::new_list(Vec::new()),
        }
    }

    pub(crate) fn equal(a: &HeaderValue, b: &HeaderValue) -> bool {
        get_address_id(a) == get_address_id(b)
    }
}
