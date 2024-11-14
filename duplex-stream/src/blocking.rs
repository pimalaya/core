use std::io::{Read, Result, Write};

use tracing::{debug, instrument, trace};

use crate::escape_byte_string;

pub const BLOCKING: bool = false;
pub type DuplexStream<S> = crate::DuplexStream<S, BLOCKING>;

impl<S: Read + Write> DuplexStream<S> {
    #[instrument(skip_all)]
    pub fn progress_read(&mut self) -> Result<usize> {
        let buf = &mut self.read_buffer;

        let byte_count = self.stream.read(buf)?;
        let byte_count = Self::validate_byte_count(byte_count)?;

        trace!(data = escape_byte_string(&buf[..byte_count]), "read");

        Ok(byte_count)
    }

    #[instrument(skip_all)]
    pub fn progress_write(&mut self) -> Result<usize> {
        let mut total_byte_count = 0;

        while self.needs_write() {
            let ref write_slices = Self::write_slices(&mut self.write_buffer);

            let byte_count = self.stream.write_vectored(write_slices)?;

            let bytes = self
                .write_buffer
                .range(..byte_count)
                .cloned()
                .collect::<Vec<_>>();

            trace!(data = escape_byte_string(bytes), "write");

            // Drop written bytes
            drop(self.write_buffer.drain(..byte_count));

            total_byte_count += Self::validate_byte_count(byte_count)?;
        }

        Ok(total_byte_count)
    }

    #[instrument(skip_all)]
    pub fn progress(&mut self) -> Result<&[u8]> {
        if self.needs_write() {
            let n = self.progress_write()?;
            debug!("wrote {n} bytes");
        }

        let n = self.progress_read()?;
        debug!("read {n} bytes");

        Ok(&self.read_buffer[..n])
    }
}

impl<S: Read + Write> Read for DuplexStream<S> {
    #[instrument(skip_all)]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let byte_count = self.get_mut().read(buf)?;
        Self::validate_byte_count(byte_count)
    }
}

impl<S: Read + Write> Write for DuplexStream<S> {
    #[instrument(skip_all)]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let byte_count = self.get_mut().write(buf)?;
        Self::validate_byte_count(byte_count)
    }

    #[instrument(skip_all)]
    fn flush(&mut self) -> Result<()> {
        self.get_mut().flush()
    }
}
