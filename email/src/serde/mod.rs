#[allow(unused_macros)]
macro_rules! serde_deprecated {
    ($name:ident, $from:literal, $to:literal) => {
        paste::paste! {
            fn [<$name _deprecated>]<'de, D: serde::Deserializer<'de>, T>(_: D) -> Result<T, D::Error> {
		let msg = format!("deprecated field {}, use {} instead", $from, $to);
                Err(serde::de::Error::custom(msg))
            }
        }
    };
}

use std::fmt;

use serde::{
    de::{Error, Visitor},
    Deserializer,
};
use shellexpand_utils::shellexpand_str;

#[allow(unused_imports)]
pub(crate) use serde_deprecated;

struct ShellExpandedStringVisitor;

impl<'de> Visitor<'de> for ShellExpandedStringVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an string containing environment variable(s)")
    }

    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(shellexpand_str(v))
    }
}

pub fn deserialize_shell_expanded_string<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    deserializer.deserialize_string(ShellExpandedStringVisitor)
}
