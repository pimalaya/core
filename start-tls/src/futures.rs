// use std::{
//     future::poll_fn,
//     io::Result,
//     pin::{pin, Pin},
//     task::Poll,
// };

// use futures::{AsyncRead, AsyncWrite};

// use crate::{PollStartTls, StartTls};

// impl<S, T> StartTls<S, T, true>
// where
//     S: AsyncRead + AsyncWrite + Unpin,
//     T: PollStartTls<S, true, Output<()> = Poll<Result<()>>> + Unpin,
// {
//     pub async fn prepare(self) -> Result<S> {
//         let p = pin!(self);
//         poll_fn(move |cx| p.starttls.poll_start_tls(stream, Some(cx))).await
//     }
// }
