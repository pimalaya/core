use async_trait::async_trait;
use std::sync::Arc;

use crate::{folder::list::ListFolders, Result};

use super::context::BackendContext;

pub trait BackendFeatures: ListFolders {
    //
}

impl<T: ListFolders> BackendFeatures for T {
    //
}

#[async_trait]
pub trait AsyncTryIntoBackendFeatures<B: BackendFeatures> {
    async fn try_into_backend(self) -> Result<B>;
}

/// The backend feature builder.
///
/// A feature builder is a function that takes an atomic reference to
/// a context as parameter and returns a backend feature.
pub type BackendFeature<C, F> = Arc<dyn Fn(&C) -> Option<Box<F>> + Send + Sync>;

#[derive(Clone, Default)]
pub enum BackendFeatureSource<C: BackendContext, F: ?Sized> {
    /// The feature should be initialized in [`Backend`] to `None`
    None,
    /// The feature should be initialized from the [`BackendContext`].
    /// If the context doesn't support this feature it will be initialized to `None`.
    #[default]
    Context,
    /// Use this given [`BackendFeatureBuilder`] to initialize the feature in [`Backend`].
    /// If this is a `BackendFeatureBuilder::none()`, it will try to initialize this feature
    /// from the context (as if [`FeatureConfiguration::Default`] was used).
    Backend(BackendFeature<C, F>),
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
