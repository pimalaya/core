use std::{
    future::poll_fn,
    io::Result,
    task::{Context, Poll},
};

use futures_io::{AsyncRead, AsyncWrite};

use crate::{Runtime, StartTls, StartTlsExt};

pub struct Async;

impl Runtime for Async {
    type Context<'a> = Context<'a>;
    type Output<T> = Poll<T>;
}

impl<S, T> StartTls<Async, S, T>
where
    S: AsyncRead + AsyncWrite + Unpin,
    T: StartTlsExt<Async, S>,
{
    pub async fn prepare(mut self) -> Result<()> {
        poll_fn(|cx| self.ext.poll(cx)).await
    }
}
