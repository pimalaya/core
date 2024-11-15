use std::io::{Cursor, Read, Result, Write};

use tracing::{debug, instrument, trace};

pub const BLOCKING: bool = false;
pub type BufStream<S> = crate::BufStream<S, BLOCKING>;

impl<S: Read + Write> BufStream<S> {
    #[instrument(skip_all)]
    fn progress_read(&mut self) -> Result<usize> {
        let slice = &mut Self::read_slice(&mut self.read_buffer);
        let count = self.stream.read_vectored(slice)?;
        Self::check_for_eof(count)?;

        let bytes = &self.read_buffer[..count];
        trace!(?bytes, len = count, "read bytes");
        Ok(count)
    }

    #[instrument(skip_all)]
    fn progress_write(&mut self) -> Result<usize> {
        let mut total_count = 0;

        while self.wants_write() {
            let write_slices = &mut Self::write_slices(&mut self.write_buffer);
            let count = self.stream.write_vectored(write_slices)?;
            total_count += Self::check_for_eof(count)?;

            let bytes = self.write_buffer.drain(..count);
            trace!(?bytes, len = count, "wrote bytes");
            drop(bytes)
        }

        Ok(total_count)
    }
}

impl<S: Read + Write> Read for BufStream<S> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut buf = Cursor::new(buf);
        let count = buf.write(&self.read_buffer[..self.read_cursor])?;
        Self::check_for_eof(count)?;
        self.fill_read_buffer(count);
        Ok(count)
    }
}

impl<S: Read + Write> Write for BufStream<S> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_buffer.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        let count = self.progress_write()?;
        debug!("wrote {count} bytes");

        self.read_cursor = self.progress_read()?;
        debug!("read {} bytes", self.read_cursor);

        self.stream.flush()
    }
}
