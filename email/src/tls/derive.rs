use serde::{Deserialize, Serialize};

#[cfg(feature = "native-tls")]
use super::NativeTls;
#[cfg(feature = "rustls")]
use super::Rustls;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum Tls {
    #[cfg(feature = "rustls")]
    Rustls(Rustls),
    #[cfg(not(feature = "rustls"))]
    #[serde(skip_serializing, deserialize_with = "missing_rustls_feature")]
    Rustls,
    #[cfg(feature = "native-tls")]
    NativeTls(NativeTls),
    #[cfg(not(feature = "native-tls"))]
    #[serde(skip_serializing, deserialize_with = "missing_native_tls_feature")]
    NativeTls,
    None,
}

impl From<Tls> for super::Tls {
    fn from(tls: Tls) -> Self {
        match tls {
            #[cfg(feature = "rustls")]
            Tls::Rustls(tls) => super::Tls::Rustls(tls),
            #[cfg(feature = "native-tls")]
            Tls::NativeTls(tls) => super::Tls::NativeTls(tls),
            _ => super::Tls::None,
        }
    }
}

#[cfg(not(feature = "rustls"))]
fn missing_rustls_feature<'de, D>(_: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    Err(Error::custom("missing `rustls` cargo feature"))
}

#[cfg(not(feature = "native-tls"))]
fn missing_native_tls_feature<'de, D>(_: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    Err(Error::custom("missing `native-tls` cargo feature"))
}
