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

#[allow(unused_imports)]
pub(crate) use serde_deprecated;
