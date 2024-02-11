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
use log::{debug, warn};
use std::{pin::Pin, sync::Arc};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::Result;

/// The thread pool task.
pub type ThreadPoolTask<C, T> =
    Box<dyn FnOnce(Arc<C>) -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync>;

/// The thread pool.
pub struct ThreadPool<C: ThreadPoolContext, T> {
    /// Channel used to send tasks to threads.
    tx: mpsc::UnboundedSender<ThreadPoolTask<C, T>>,

    /// Channel used to receive tasks output.
    rx: mpsc::UnboundedReceiver<T>,

    /// The list of threads spawned by the pool.
    threads: Vec<JoinHandle<Result<()>>>,
}

impl<C, T> ThreadPool<C, T>
where
    C: ThreadPoolContext + 'static,
    T: Send + 'static,
{
    /// Send a task to the pool.
    pub fn send<F>(&mut self, task: impl FnOnce(Arc<C>) -> F + Send + Sync + 'static) -> Result<()>
    where
        F: Future<Output = T> + Send + 'static,
    {
        let task: ThreadPoolTask<C, T> = Box::new(move |ctx| Box::pin(task(ctx)));
        self.tx.send(task)?;
        Ok(())
    }

    /// Receive a task output from the pool.
    pub async fn recv(&mut self) -> Option<T> {
        self.rx.recv().await
    }

    /// Close channels and abort threads.
    pub fn shutdown(mut self) {
        self.rx.close();
        for thread in self.threads {
            thread.abort()
        }
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
    pub async fn build<T>(self) -> Result<ThreadPool<B::Context, T>>
    where
        T: Send + 'static,
    {
        // channel for workers to receive and process tasks from the pool
        let (tx_pool, rx_worker) = mpsc::unbounded_channel::<ThreadPoolTask<B::Context, T>>();
        let rx_workers = Arc::new(Mutex::new(rx_worker));

        // channel for workers to send output of their work to the pool
        let (tx_worker, rx_pool) = mpsc::unbounded_channel::<T>();

        let mut threads = Vec::with_capacity(self.size);

        let ctxs = FuturesUnordered::from_iter(
            (0..self.size).map(|_| async { self.ctx_builder.clone().build().await }),
        )
        .collect::<Vec<Result<_>>>()
        .await;

        for (mut id, ctx) in ctxs.into_iter().enumerate() {
            id += 1;

            let ctx = ctx?;
            let tx = tx_worker.clone();
            let rx = rx_workers.clone();

            threads.push(tokio::spawn(async move {
                let ctx = Arc::new(ctx);

                loop {
                    let mut lock = rx.lock().await;

                    debug!("thread {id} waiting for a task…");
                    match lock.recv().await {
                        None => {
                            drop(lock);
                            break;
                        }
                        Some(task) => {
                            drop(lock);

                            debug!("thread {id} received a task, executing it…");
                            let output = task(ctx.clone()).await;
                            debug!("thread {id} successfully executed task!");

                            if let Err(err) = tx.send(output) {
                                warn!("thread {id} cannot send task output: {err}");
                            }
                        }
                    }
                }

                debug!("no more task for thread {id}, exitting");

                Result::Ok(())
            }));
        }

        Ok(ThreadPool {
            tx: tx_pool,
            rx: rx_pool,
            threads,
        })
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
