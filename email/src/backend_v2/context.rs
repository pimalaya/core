use async_trait::async_trait;
use paste::paste;

use crate::{folder::list::ListFolders, Result};

use super::feature::BackendFeature;

macro_rules! feature_setter {
    ($feat:ty) => {
        paste! {
            fn [<$feat:snake>](&self) -> Option<BackendFeature<Self::Context, dyn $feat>> {
                None
            }
        }
    };
}

#[async_trait]
pub trait BackendContextBuilder: Clone + Send + Sync {
    /// The type of the context being built by the builder.
    ///
    /// The context needs to implement [`Send`], as it is sent accross
    /// asynchronous tasks. Wrapping your context in a
    /// [`std::sync::Arc`] should be enough. If your context needs to
    /// be mutated, you can also wrap it in a
    /// [`tokio::sync::Mutex`]. See existing implementations of
    /// `email::imap::ImapContextSync` or
    /// `email::smtp::SmtpContextSync`.
    type Context: BackendContext;

    feature_setter!(ListFolders);

    /// Build the final context.
    async fn build(self) -> Result<Self::Context>;
}

/// The backend context trait.
///
/// This is just a marker for other traits. Every backend context
/// needs to implement this trait manually or to derive
/// [`email_macros::BackendContext`].
pub trait BackendContext: Send + Sync {
    //
}
