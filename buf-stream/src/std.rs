use std::io::{Read, Result, Write};

use tracing::{debug, instrument, trace};

pub const BLOCKING: bool = false;
pub type BufStream<S> = crate::BufStream<S, BLOCKING>;

impl<S: Read> BufStream<S> {
    #[instrument(skip_all)]
    pub fn progress_read(&mut self) -> Result<usize> {
        let buf = &mut self.read_buf;

        let byte_count = self.stream.read(buf)?;
        let byte_count = Self::validate_byte_count(byte_count)?;

        trace!(data = ?buf[..byte_count], "read");

        Ok(byte_count)
    }
}

impl<S: Write> BufStream<S> {
    #[instrument(skip_all)]
    pub fn progress_write(&mut self) -> Result<usize> {
        let mut total_byte_count = 0;

        while self.needs_write() {
            let ref write_slices = Self::write_slices(&mut self.write_buf);

            let byte_count = self.stream.write_vectored(write_slices)?;

            let bytes = self.write_buf.drain(..byte_count);
            trace!(data = ?bytes, "write");

            drop(bytes);

            total_byte_count += Self::validate_byte_count(byte_count)?;
        }

        Ok(total_byte_count)
    }
}

impl<S: Read + Write> BufStream<S> {
    #[instrument(skip_all)]
    pub fn progress(&mut self) -> Result<&[u8]> {
        if self.needs_write() {
            let n = self.progress_write()?;
            debug!("wrote {n} bytes");
        }

        let n = self.progress_read()?;
        debug!("read {n} bytes");

        Ok(&self.read_buf[..n])
    }
}
