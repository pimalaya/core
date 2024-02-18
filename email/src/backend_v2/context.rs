//! # Backend context
//!
//! The [`BackendContext`] is usually used for storing clients or
//! sessions (structures than cannot be cloned or sync). The
//! [`BackendContextBuilder`] gives instructions on how to build such
//! context. It is used by the backend builder.

use async_trait::async_trait;
use paste::paste;

use crate::{folder::list::ListFolders, Result};

use super::feature::BackendFeature;

/// The backend context.
///
/// This is just a marker for other backend traits. Every backend
/// context needs to implement this trait manually or to derive
/// [`crate::backend_v2::macros::BackendContextV2`].
pub trait BackendContext: Send + Sync {
    //
}

/// Macro for defining [`BackendContextBuilder`] feature setter.
macro_rules! feature_setter {
    ($feat:ty) => {
        paste! {
            /// Define the given backend feature.
            fn [<$feat:snake>](&self) -> Option<BackendFeature<Self::Context, dyn $feat>> {
                None
            }
        }
    };
}

/// The backend context builder.
///
/// This trait defines how a context should be built. It also defines
/// default backend features implemented by the context itself.
#[async_trait]
pub trait BackendContextBuilder: Clone + Send + Sync {
    /// The type of the context being built by this builder.
    type Context: BackendContext;

    feature_setter!(ListFolders);

    /// Build the final context used by the backend.
    async fn build(self) -> Result<Self::Context>;
}
