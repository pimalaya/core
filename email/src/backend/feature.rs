//! # Backend feature
//!
//! A [`BackendFeature`] is an action like adding folder, listing
//! envelopes or sending message. A feature needs a backend context to
//! be executed.

use std::sync::Arc;

use async_trait::async_trait;

use super::{context::BackendContext, AnyResult};

/// Backend builder feature for checking up configuration and context
/// integrity.
///
/// This feature is used to check the integrity of the context.
#[async_trait]
pub trait CheckUp: Send + Sync {
    /// Define how the no operation should be executed.
    async fn check_up(&self) -> AnyResult<()> {
        Ok(())
    }
}

/// The backend feature.
///
/// A backend feature is a function that takes a reference to a
/// backend context as parameter and returns a feature.
pub type BackendFeature<C, F> = Arc<dyn Fn(&C) -> Option<Box<F>> + Send + Sync>;

/// The backend feature source.
///
/// This enum is used by the backend builder to determine where a
/// specific backend feature should be taken from.
#[derive(Default)]
pub enum BackendFeatureSource<C: BackendContext, F: ?Sized> {
    /// The feature should be disabled.
    None,

    /// The feature should be taken from the
    /// [`super::BackendContextBuilder`].
    #[default]
    Context,

    /// The feature should be taken from the
    /// [`super::BackendBuilder`], using the given feature.
    Backend(BackendFeature<C, F>),
}

impl<C, F> Clone for BackendFeatureSource<C, F>
where
    C: BackendContext,
    F: ?Sized,
{
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Context => Self::Context,
            Self::Backend(f) => Self::Backend(f.clone()),
        }
    }
}

impl<C, F, T> From<T> for BackendFeatureSource<C, F>
where
    C: BackendContext,
    F: ?Sized,
    T: Fn(&C) -> Option<Box<F>> + Send + Sync + 'static,
{
    fn from(value: T) -> Self {
        Self::Backend(Arc::new(value))
    }
}
