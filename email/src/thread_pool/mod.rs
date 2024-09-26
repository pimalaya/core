//! # Thread pool
//!
//! Module dedicated to thread pool management. The [`ThreadPool`] is
//! the main structure of this module: it basically spawns n threads
//! and transfers tasks to them using an unbounded channel. The
//! receiver part is shared accross all threads in a mutex, this way
//! only one thread can wait for a task at a time. When a thread
//! receives a task, it releases the lock and an other thread can wait
//! for the next task. A task is a function that takes a
//! [`ThreadPoolContextBuilder::Context`] and returns a future. The
//! easiest way to build a pool is to use the [`ThreadPoolBuilder`].

mod error;

use std::{
    num::NonZeroUsize,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
    thread::available_parallelism,
    time::Duration,
};

use async_trait::async_trait;
use futures::{lock::Mutex, stream::FuturesUnordered, Future, StreamExt};
use tokio::{task::JoinHandle, time::sleep};

#[doc(inline)]
pub use self::error::{Error, Result};
use crate::AnyResult;

/// The thread pool task.
pub type ThreadPoolTask<C> =
    Box<dyn FnOnce(Arc<C>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// The thread pool task resolver.
///
/// The resolver awaits for a task to be executed by one of the
/// available thread of the pool using a shared state.
pub struct ThreadPoolTaskResolver<T>(Arc<Mutex<ThreadPoolTaskResolverState<T>>>);

impl<T> ThreadPoolTaskResolver<T> {
    /// Create a new task resolver with an empty state.
    pub fn new() -> Self {
        Default::default()
    }

    /// Resolves the given task.
    ///
    /// The task output is saved into the shared state, and if a waker
    /// is found in the shared state the resolver is polled again.
    pub async fn resolve(&self, task: impl Future<Output = T> + Send + 'static) {
        let output = task.await;
        let mut state = self.0.lock().await;
        state.output = Some(output);
        if let Some(waker) = state.waker.take() {
            waker.wake()
        }
    }
}

impl<T> Default for ThreadPoolTaskResolver<T> {
    fn default() -> Self {
        let state = ThreadPoolTaskResolverState {
            output: None,
            waker: None,
        };

        Self(Arc::new(Mutex::new(state)))
    }
}

impl<T> Clone for ThreadPoolTaskResolver<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Future for ThreadPoolTaskResolver<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Box::pin(self.0.lock()).as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(mut state) => match state.output.take() {
                Some(output) => Poll::Ready(output),
                None => {
                    state.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            },
        }
    }
}

/// The thread pool task resolver shared state.
pub struct ThreadPoolTaskResolverState<T> {
    /// The output of the resolved task.
    output: Option<T>,

    /// The resolver waker.
    waker: Option<Waker>,
}

/// The thread pool.
pub struct ThreadPool<C: ThreadPoolContext> {
    tasks: Arc<Mutex<Vec<ThreadPoolTask<C>>>>,

    /// The list of threads spawned by the pool.
    threads: Vec<JoinHandle<Result<()>>>,
}

impl<C> ThreadPool<C>
where
    C: ThreadPoolContext + 'static,
{
    /// Execute the given task and awaits for its resolution.
    ///
    /// The task is sent to the pool channel and will be executed by
    /// the first available thread. This function awaits for its
    /// resolution.
    pub async fn exec<F, T>(&self, task: impl FnOnce(Arc<C>) -> F + Send + Sync + 'static) -> T
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let resolver = ThreadPoolTaskResolver::new();

        let r = resolver.clone();
        let task: ThreadPoolTask<C> = Box::new(move |ctx| {
            let task = async move { r.resolve(task(ctx)).await };
            Box::pin(task)
        });

        {
            let mut tasks = self.tasks.lock().await;
            tasks.push(task);
        }

        resolver.await
    }

    /// Abort pool threads and close the channel.
    pub async fn close(self) {
        #[cfg(feature = "tracing")]
        tracing::debug!("closing pool…");

        for thread in &self.threads {
            thread.abort()
        }

        for (id, thread) in self.threads.into_iter().enumerate() {
            let id = id + 1;

            #[cfg(not(feature = "tracing"))]
            let _ = thread.await;

            #[cfg(feature = "tracing")]
            match thread.await {
                Ok(_) => tracing::debug!(id, "thread aborted"),
                Err(err) => tracing::debug!(id, info = err.to_string(), "thread aborted"),
            }
        }

        let mut tasks = self.tasks.lock().await;

        #[cfg(feature = "tracing")]
        tracing::debug!(size = tasks.len(), "cleaning remaining tasks");

        tasks.clear();

        #[cfg(feature = "tracing")]
        tracing::debug!("pool closed");
    }
}

/// The thread pool builder.
///
/// Builder that help you to create a [`ThreadPool`].
#[derive(Clone)]
pub struct ThreadPoolBuilder<B: ThreadPoolContextBuilder> {
    /// The context builder.
    ctx_builder: B,

    /// The size of the pool.
    ///
    /// Represents the number of threads that will be spawn in
    /// parallel. Defaults to the number of available CPUs.
    size: usize,
}

impl<B: ThreadPoolContextBuilder + 'static> ThreadPoolBuilder<B> {
    /// Create a new thread pool builder with a context builder.
    pub fn new(ctx_builder: B) -> Self {
        Self {
            ctx_builder,
            size: available_parallelism().map_or(1, NonZeroUsize::get),
        }
    }

    /// Change the thread pool size.
    pub fn set_some_size(&mut self, size: Option<usize>) {
        if let Some(size) = size {
            self.size = size;
        }
    }

    /// Change the thread pool size.
    pub fn set_size(&mut self, size: usize) {
        self.set_some_size(Some(size));
    }

    /// Change the thread pool size using the builder pattern.
    pub fn with_some_size(mut self, size: Option<usize>) -> Self {
        self.set_some_size(size);
        self
    }

    /// Change the thread pool size using the builder pattern.
    pub fn with_size(mut self, size: usize) -> Self {
        self.set_size(size);
        self
    }

    /// Build the final thread pool.
    pub async fn build(self) -> Result<ThreadPool<B::Context>> {
        let tasks = Arc::new(Mutex::new(Vec::<ThreadPoolTask<B::Context>>::new()));

        #[cfg(feature = "tracing")]
        tracing::debug!(size = self.size, "creating pool");

        let ctxs = FuturesUnordered::from_iter(
            (0..self.size).map(move |_| tokio::spawn(self.ctx_builder.clone().build())),
        )
        .collect::<Vec<_>>()
        .await;

        let mut threads = Vec::with_capacity(self.size);

        for (id, ctx) in ctxs.into_iter().enumerate() {
            let id = id + 1;
            let tasks = tasks.clone();

            let ctx = ctx?.map_err(|err| Error::BuildContextError(err, id, self.size))?;
            let ctx = Arc::new(ctx);

            threads.push(tokio::spawn(async move {
                let ctx = ctx.clone();

                loop {
                    #[cfg(feature = "tracing")]
                    tracing::trace!(id, "thread looking for a task");

                    let mut lock = tasks.try_lock();
                    let task = lock.as_mut().and_then(|tasks| tasks.pop());
                    drop(lock);

                    match task {
                        None => {
                            #[cfg(feature = "tracing")]
                            tracing::debug!(id, "no task available, sleeping for 1s");
                            sleep(Duration::from_secs(1)).await;
                        }
                        Some(task) => {
                            #[cfg(feature = "tracing")]
                            tracing::debug!(id, "thread executing task…");

                            task(ctx.clone()).await;

                            #[cfg(feature = "tracing")]
                            tracing::debug!(id, "thread successfully executed task");
                        }
                    }
                }
            }));
        }

        Ok(ThreadPool { tasks, threads })
    }
}

/// The thread pool context builder.
#[async_trait]
pub trait ThreadPoolContextBuilder: Clone + Send + Sync {
    /// The context built by this trait.
    type Context: ThreadPoolContext;

    /// Build the thread pool context.
    async fn build(self) -> AnyResult<Self::Context>;
}

pub trait ThreadPoolContext: Send + Sync {
    //
}
