#[cfg(feature = "async")]
pub mod futures;
#[cfg(feature = "blocking")]
pub mod std;

use ::std::{
    collections::VecDeque,
    io::{Error, ErrorKind, IoSlice, Result},
};

pub struct BufStream<S, const MAYBE_ASYNC: bool> {
    stream: S,
    read_buf: Box<[u8]>,
    write_buf: VecDeque<u8>,
}

impl<S, const MAYBE_ASYNC: bool> BufStream<S, MAYBE_ASYNC> {
    pub fn new(stream: S) -> Self {
        Self::new_with_capacity(stream, 1024)
    }

    pub fn new_with_capacity(stream: S, capacity: usize) -> Self {
        Self {
            stream,
            read_buf: vec![0; capacity].into(),
            write_buf: VecDeque::new(),
        }
    }

    pub fn read_buffer(&mut self) -> &[u8] {
        &self.read_buf
    }

    pub fn write_buffer(&mut self) -> &VecDeque<u8> {
        &self.write_buf
    }

    pub fn push_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        self.write_buf.extend(bytes.as_ref());
    }

    fn needs_write(&self) -> bool {
        !self.write_buf.is_empty()
    }

    fn write_slices(buf: &VecDeque<u8>) -> [IoSlice; 2] {
        let (init, tail) = buf.as_slices();
        [IoSlice::new(init), IoSlice::new(tail)]
    }
}

impl<S, const MAYBE_ASYNC: bool> BufStream<S, MAYBE_ASYNC> {
    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    pub fn into_inner(self) -> S {
        self.stream
    }

    fn validate_byte_count(byte_count: usize) -> Result<usize> {
        if byte_count == 0 {
            // The result is 0 if the stream doesn't accept bytes anymore or the write buffer
            // was already empty before calling `write_buf`. Because we checked the buffer
            // we know that the first case occurred.
            let err = Error::new(ErrorKind::UnexpectedEof, "received empty bytes");
            return Err(err);
        }

        Ok(byte_count)
    }
}
