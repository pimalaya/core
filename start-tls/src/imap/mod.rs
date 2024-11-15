#[cfg(feature = "async")]
pub mod futures;
#[cfg(feature = "blocking")]
pub mod std;

pub struct ImapStartTls<'a, S, const IS_ASYNC: bool> {
    stream: &'a mut S,
    buf: Vec<u8>,
    command_sent: bool,
}

impl<'a, S, const IS_ASYNC: bool> AsMut<S> for ImapStartTls<'a, S, IS_ASYNC> {
    fn as_mut(&mut self) -> &mut S {
        &mut self.stream
    }
}

impl<'a, S, const IS_ASYNC: bool> ImapStartTls<'a, S, IS_ASYNC> {
    const COMMAND: &'static str = "A1 STARTTLS\r\n";

    pub fn new(stream: &'a mut S) -> Self {
        Self {
            stream,
            buf: vec![0; 512],
            command_sent: false,
        }
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        self.buf = vec![0; capacity];
    }

    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.set_capacity(capacity);
        self
    }
}
