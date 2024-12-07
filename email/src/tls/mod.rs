use std::fmt;

#[cfg(feature = "derive")]
pub mod derive;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case"),
    serde(tag = "type", content = "provider")
)]
pub enum Encryption {
    Tls(Tls),
    StartTls(Tls),
    None,
}

impl Default for Encryption {
    fn default() -> Self {
        Self::Tls(Default::default())
    }
}

impl fmt::Display for Encryption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tls(_) => write!(f, "SSL/TLS"),
            Self::StartTls(_) => write!(f, "StartTLS"),
            Self::None => write!(f, "None"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case"),
    serde(tag = "type"),
    serde(from = "derive::Tls")
)]
pub enum Tls {
    #[cfg(feature = "rustls")]
    Rustls(Rustls),
    #[cfg(feature = "native-tls")]
    NativeTls(NativeTls),
    None,
}

#[cfg(feature = "rustls")]
impl Default for Tls {
    fn default() -> Self {
        Tls::Rustls(Default::default())
    }
}

#[cfg(not(feature = "rustls"))]
impl Default for Tls {
    fn default() -> Self {
        Tls::None
    }
}

impl fmt::Display for Tls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "rustls")]
            Self::Rustls(_) => write!(f, "Rust native (rustls)"),
            #[cfg(feature = "native-tls")]
            Self::NativeTls(_) => write!(f, "OS native (native-tls)"),
            Self::None => write!(f, "None"),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg(feature = "rustls")]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct Rustls {
    // TODO: define rustls specific options?
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg(feature = "native-tls")]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct NativeTls {
    // TODO: define native-tls specific options?
}
