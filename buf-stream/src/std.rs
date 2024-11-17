use std::io::{Cursor, Read, Result, Write};

use tracing::debug;

use crate::{ReadBuffer, WriteBuffer};

pub struct BufStream<S> {
    stream: S,
    read_buffer: ReadBuffer,
    write_buffer: WriteBuffer,
}

impl<S> BufStream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            read_buffer: Default::default(),
            write_buffer: Default::default(),
        }
    }

    pub fn set_read_capacity(&mut self, capacity: usize) {
        self.read_buffer.set_capacity(capacity)
    }

    pub fn with_read_capacity(mut self, capacity: usize) -> Self {
        self.read_buffer.set_capacity(capacity);
        self
    }

    pub fn wants_read(&self) -> bool {
        self.read_buffer.wants_read()
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
}

impl<S: Read + Write> BufStream<S> {
    pub fn progress_read(&mut self) -> Result<usize> {
        let slice = &mut self.read_buffer.to_io_slice_mut();
        let count = self.stream.read_vectored(slice)?;
        self.read_buffer.progress(count)
    }

    pub fn progress_write(&mut self) -> Result<usize> {
        if !self.write_buffer.wants_write() {
            return Ok(0);
        }

        let slices = &mut self.write_buffer.to_io_slices();
        let count = self.stream.write_vectored(slices)?;
        self.write_buffer.progress(count)
    }

    pub fn progress(&mut self) -> Result<&[u8]> {
        let count = self.progress_write()?;
        debug!("wrote {count} bytes");

        let count = self.progress_read()?;
        debug!("read {count} bytes");

        self.stream.flush()?;

        Ok(&self.read_buffer.as_slice()[..count])
    }
}

impl<S: Read + Write> Read for BufStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.read_buffer.wants_read() {
            return Ok(0);
        }

        let mut buf = Cursor::new(buf);
        let count = buf.write(self.read_buffer.as_slice())?;
        self.read_buffer.sync(count)
    }
}

impl<S: Read + Write> Write for BufStream<S> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_buffer.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        self.progress_write()?;
        self.stream.flush()
    }
}
