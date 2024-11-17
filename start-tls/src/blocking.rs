use ::std::io::Result;

pub trait PrepareStartTls<S> {
    fn prepare(self, stream: &mut S) -> Result<()>;
}
