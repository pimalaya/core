use ::std::marker::PhantomData;

#[cfg(feature = "async")]
pub mod futures;
pub mod imap;
pub mod smtp;
#[cfg(feature = "blocking")]
pub mod std;

pub trait StartTlsExt<S, const IS_ASYNC: bool> {
    type Context<'a>;
    type Output<T>;

    fn poll(&mut self, cx: &mut Self::Context<'_>) -> Self::Output<()>;
}

pub struct StartTls<S, T: StartTlsExt<S, IS_ASYNC>, const IS_ASYNC: bool> {
    stream: PhantomData<S>,
    ext: T,
}

impl<S, T: StartTlsExt<S, IS_ASYNC>, const IS_ASYNC: bool> StartTls<S, T, IS_ASYNC> {
    pub fn new(ext: T) -> Self {
        Self {
            stream: PhantomData::default(),
            ext,
        }
    }
}
