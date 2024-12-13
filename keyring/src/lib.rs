#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

pub mod event;
pub mod state;

#[cfg(feature = "secret-service-blocking")]
#[cfg(any(target_os = "linux", debug_assertions))]
pub mod secret_service_blocking;
#[cfg(feature = "secret-service-nonblock")]
#[cfg(any(target_os = "linux", debug_assertions))]
pub mod secret_service_nonblock;
#[cfg(feature = "security-framework")]
#[cfg(any(target_os = "macos", debug_assertions))]
pub mod security_framework;
