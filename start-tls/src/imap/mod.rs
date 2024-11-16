use ::std::marker::PhantomData;

use crate::Runtime;

#[cfg(feature = "async")]
pub mod futures;
#[cfg(feature = "blocking")]
pub mod std;

pub struct ImapStartTls<'a, R: Runtime, S> {
    runtime: PhantomData<R>,
    stream: &'a mut S,
    buf: Vec<u8>,
    handshake_discarded: bool,
    command_sent: bool,
}

impl<'a, R: Runtime, S> ImapStartTls<'a, R, S> {
    const COMMAND: &'static str = "A1 STARTTLS\r\n";

    pub fn new(stream: &'a mut S) -> Self {
        Self {
            runtime: PhantomData::default(),
            stream,
            buf: vec![0; 512],
            handshake_discarded: false,
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
