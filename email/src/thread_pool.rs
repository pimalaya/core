//! # Thread pool
//!
//! Module dedicated to thread pool management. The [`ThreadPool`]
//! struct allows you to send tasks and receive task output from the
//! pool. A task is a function that takes a
//! [`ThreadPoolContextBuilder::Context`] and returns a future. The
//! future resolves to a task output (usually an enum). The easiest
//! way to build a pool is to use the [`ThreadPoolBuilder`].

use async_trait::async_trait;
use futures::{lock::Mutex, stream::FuturesUnordered, Future, StreamExt};
use log::debug;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::Result;

/// The thread pool task.
pub type ThreadPoolTask<C> =
    Box<dyn FnOnce(Arc<C>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// The thread pool task resolver.
///
/// The resolver awaits for a task to be executed using a shared
/// state.
pub struct ThreadPoolTaskResolver<T>(Arc<Mutex<ThreadPoolTaskResolverState<T>>>);

impl<T> ThreadPoolTaskResolver<T> {
    /// Create a new task resolver with an empty state.
    pub fn new() -> Self {
        let state = ThreadPoolTaskResolverState {
            output: None,
            waker: None,
        };

        Self(Arc::new(Mutex::new(state)))
    }

    /// Resolves the given task.
    ///
    /// The task output is saved into the shared state, and poll again
    /// the resolver if a waker is found in the shared state.
    pub async fn resolve(&self, task: impl Future<Output = T> + Send + 'static) {
        let output = task.await;
        let mut state = self.0.lock().await;
        state.output = Some(output);
        if let Some(waker) = state.waker.take() {
            waker.wake()
        }
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
    /// Channel used by the pool.
    ///
    /// This channel is used to send tasks to threads.
    tx: mpsc::UnboundedSender<ThreadPoolTask<C>>,

    /// Channel used by threads.
    ///
    /// Only one thread can receive a task at a time. When a thread
    /// receives a task, it releases the lock and a new thread can
    /// receive the next task.
    rx: Arc<Mutex<mpsc::UnboundedReceiver<ThreadPoolTask<C>>>>,

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

        self.tx.send(task).unwrap();

        resolver.await
    }

    /// Abort pool threads and close the channel receiver.
    pub async fn shutdown(self) {
        for thread in self.threads {
            thread.abort()
        }

        self.rx.lock().await.close();
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
    /// parallel. Defaults to 8.
    size: usize,
}

impl<B: ThreadPoolContextBuilder + 'static> ThreadPoolBuilder<B> {
    /// Create a new thread pool builder with a context builder.
    pub fn new(ctx_builder: B) -> Self {
        Self {
            ctx_builder,
            size: 8,
        }
    }

    /// Change the thread pool size.
    pub fn set_size(&mut self, size: usize) {
        self.size = size;
    }

    /// Change the thread pool size using the builder pattern.
    pub fn with_size(mut self, size: usize) -> Self {
        self.set_size(size);
        self
    }

    /// Build the final thread pool.
    pub async fn build(self) -> Result<ThreadPool<B::Context>> {
        let (tx, rx) = mpsc::unbounded_channel::<ThreadPoolTask<B::Context>>();
        let rx = Arc::new(Mutex::new(rx));

        let mut threads = Vec::with_capacity(self.size);

        let ctxs = FuturesUnordered::from_iter(
            (0..self.size).map(|_| async { self.ctx_builder.clone().build().await }),
        )
        .collect::<Vec<Result<_>>>()
        .await;

        for (index, ctx) in ctxs.into_iter().enumerate() {
            let ctx = ctx?;
            let thread_id = index + 1;
            let rx = rx.clone();

            threads.push(tokio::spawn(async move {
                let ctx = Arc::new(ctx);

                loop {
                    let mut lock = rx.lock().await;

                    debug!("thread {thread_id} waiting for a task…");
                    match lock.recv().await {
                        None => {
                            drop(lock);
                            break;
                        }
                        Some(task) => {
                            drop(lock);

                            debug!("thread {thread_id} received a task, executing it…");
                            task(ctx.clone()).await;
                            debug!("thread {thread_id} successfully executed task!");
                        }
                    }
                }

                debug!("no more task for thread {thread_id}, exitting");

                Result::Ok(())
            }));
        }

        Ok(ThreadPool { tx, rx, threads })
    }
}

/// The thread pool context builder.
#[async_trait]
pub trait ThreadPoolContextBuilder: Clone + Send + Sync {
    /// The context built by this trait.
    type Context: ThreadPoolContext;

    /// Build the thread pool context.
    async fn build(self) -> Result<Self::Context>;
}

pub trait ThreadPoolContext: Send + Sync {
    //
}
