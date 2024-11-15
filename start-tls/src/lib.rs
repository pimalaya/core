use ::std::task::Context;

#[cfg(feature = "async")]
pub mod futures;
pub mod imap;
#[cfg(feature = "blocking")]
pub mod std;

pub trait PollStartTls<S, const IS_ASYNC: bool>: AsMut<S> {
    type Output<T>;

    fn poll_start_tls(&mut self, cx: Option<&mut Context<'_>>) -> Self::Output<()>;
}

// pub struct StartTls<S, T: PollStartTls<S, IS_ASYNC>, const IS_ASYNC: bool> {
//     stream: S,
//     starttls: T,
// }
