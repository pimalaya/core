#[cfg(feature = "async")]
pub mod futures;
#[cfg(feature = "blocking")]
pub mod std;

use ::std::{
    collections::VecDeque,
    io::{Error, ErrorKind, IoSlice, IoSliceMut, Result},
};

use tracing::debug;

pub struct BufStream<S, const MAYBE_ASYNC: bool> {
    stream: S,
    read_buffer: Box<[u8]>,
    read_cursor: usize,
    write_buffer: VecDeque<u8>,
}

impl<S, const MAYBE_ASYNC: bool> BufStream<S, MAYBE_ASYNC> {
    pub fn new(stream: S) -> Self {
        Self::with_capacity(stream, 1024)
    }

    pub fn with_capacity(stream: S, capacity: usize) -> Self {
        Self {
            stream,
            read_buffer: vec![0; capacity].into(),
            read_cursor: 0,
            write_buffer: VecDeque::new(),
        }
    }

    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    pub fn into_inner(self) -> S {
        self.stream
    }

    pub fn wants_read(&self) -> bool {
        self.read_cursor > 0
    }
}

impl<S, const MAYBE_ASYNC: bool> BufStream<S, MAYBE_ASYNC> {
    fn wants_write(&self) -> bool {
        !self.write_buffer.is_empty()
    }

    fn read_slice(buf: &mut Box<[u8]>) -> [IoSliceMut; 1] {
        [IoSliceMut::new(buf.as_mut())]
    }

    fn write_slices(buf: &VecDeque<u8>) -> [IoSlice; 2] {
        let (init, tail) = buf.as_slices();
        [IoSlice::new(init), IoSlice::new(tail)]
    }

    fn check_for_eof(count: usize) -> Result<usize> {
        if count == 0 {
            let err = Error::new(ErrorKind::UnexpectedEof, "received empty bytes");
            return Err(err);
        }

        Ok(count)
    }

    fn fill_read_buffer(&mut self, count: usize) {
        debug!("read {count}/{} bytes", self.read_cursor);
        let remaining = self.read_buffer.len() - count;
        self.read_buffer.copy_within(count.., 0);
        self.read_buffer[remaining..].fill(0);
        self.read_cursor -= count;
    }
}
