#[cfg(feature = "async")]
pub mod futures;
#[cfg(feature = "blocking")]
pub mod std;

use ::std::{
    collections::VecDeque,
    io::{Error, ErrorKind, IoSlice, IoSliceMut, Result},
};

use tracing::{debug, trace};

#[derive(Clone, Debug)]
pub(crate) struct ReadBuffer {
    buffer: Box<[u8]>,
    cursor: usize,
}

impl ReadBuffer {
    fn new() -> Self {
        Self {
            buffer: vec![0; 1024].into(),
            cursor: 0,
        }
    }

    fn set_capacity(&mut self, capacity: usize) {
        self.buffer = vec![0; capacity].into();
    }

    fn wants_read(&self) -> bool {
        self.cursor > 0 && !self.buffer.is_empty()
    }

    fn to_io_slice_mut(&mut self) -> [IoSliceMut; 1] {
        [IoSliceMut::new(self.buffer.as_mut())]
    }

    fn as_slice(&self) -> &[u8] {
        &self.buffer.as_ref()[..self.cursor]
    }

    fn sync(&mut self, count: usize) -> Result<usize> {
        validate_byte_count(count)?;
        debug!("read {count}/{} bytes", self.cursor);
        let remaining = self.buffer.len() - count;
        self.buffer.copy_within(count.., 0);
        self.buffer[remaining..].fill(0);
        self.cursor -= count;
        Ok(count)
    }

    fn progress(&mut self, count: usize) -> Result<usize> {
        self.cursor = validate_byte_count(count)?;
        let bytes = &self.buffer[..self.cursor];
        trace!(?bytes, len = self.cursor, "read bytes");
        Ok(self.cursor)
    }
}

impl Default for ReadBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct WriteBuffer {
    buffer: VecDeque<u8>,
}

impl WriteBuffer {
    fn wants_write(&self) -> bool {
        !self.buffer.is_empty()
    }

    fn to_io_slices(&self) -> [IoSlice; 2] {
        let (init, tail) = self.buffer.as_slices();
        [IoSlice::new(init), IoSlice::new(tail)]
    }

    fn extend(&mut self, bytes: &[u8]) {
        self.buffer.extend(bytes)
    }

    fn progress(&mut self, count: usize) -> Result<usize> {
        validate_byte_count(count)?;
        let bytes = self.buffer.drain(..count);
        trace!(?bytes, len = count, "wrote bytes");
        drop(bytes);
        Ok(count)
    }
}

fn validate_byte_count(count: usize) -> Result<usize> {
    if count == 0 {
        let err = Error::new(ErrorKind::UnexpectedEof, "received empty bytes");
        return Err(err);
    }

    Ok(count)
}
