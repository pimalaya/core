use ::std::io::Result;

pub trait StartTlsExt<S> {
    fn prepare(self, stream: &mut S) -> Result<()>;
}
