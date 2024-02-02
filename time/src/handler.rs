use std::{future::Future, io::Result, pin::Pin, sync::Arc};

pub(crate) type Handler<E> =
    dyn Fn(E) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync;

pub(crate) fn default<E>() -> Arc<Handler<E>> {
    Arc::new(|_| Box::pin(async { Ok(()) }))
}
