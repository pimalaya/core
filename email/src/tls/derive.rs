use serde::{Deserialize, Serialize};

#[cfg(feature = "native-tls")]
use super::NativeTls;
#[cfg(feature = "rustls")]
use super::Rustls;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum TlsProvider {
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

impl From<TlsProvider> for super::TlsProvider {
    fn from(provider: TlsProvider) -> Self {
        match provider {
            #[cfg(feature = "rustls")]
            TlsProvider::Rustls(provider) => super::TlsProvider::Rustls(provider),
            #[cfg(feature = "native-tls")]
            TlsProvider::NativeTls(provider) => super::TlsProvider::NativeTls(provider),
            _ => super::TlsProvider::None,
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
