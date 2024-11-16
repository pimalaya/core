use std::{
    future::poll_fn,
    io::Result,
    task::{Context, Poll},
};

use futures_io::{AsyncRead, AsyncWrite};

use crate::{StartTls, StartTlsExt};

impl<S, T> StartTls<S, T, true>
where
    S: AsyncRead + AsyncWrite + Unpin,
    T: for<'a> StartTlsExt<S, true, Context<'a> = Context<'a>, Output<()> = Poll<Result<()>>>,
{
    pub async fn prepare(mut self) -> Result<()> {
        poll_fn(|cx| self.ext.poll(cx)).await
    }
}
