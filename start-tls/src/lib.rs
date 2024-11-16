use ::std::{io::Result, marker::PhantomData};

#[cfg(feature = "async")]
pub mod futures;
pub mod imap;
pub mod smtp;
#[cfg(feature = "blocking")]
pub mod std;

pub trait Runtime {
    type Context<'a>;
    type Output<T>;
}

pub trait StartTlsExt<R: Runtime, S> {
    fn poll(&mut self, cx: &mut R::Context<'_>) -> R::Output<Result<()>>;
}

pub struct StartTls<R: Runtime, S, T: StartTlsExt<R, S>> {
    runtime: PhantomData<R>,
    stream: PhantomData<S>,
    ext: T,
}

impl<R: Runtime, S, T: StartTlsExt<R, S>> StartTls<R, S, T> {
    pub fn new(ext: T) -> Self {
        Self {
            runtime: PhantomData::default(),
            stream: PhantomData::default(),
            ext,
        }
    }
}
