#[cfg(feature = "async")]
pub mod r#async;
#[cfg(feature = "blocking")]
pub mod blocking;

use ::std::{collections::VecDeque, io::IoSlice};

pub struct DuplexStream<S, const IS_ASYNC: bool> {
    stream: S,
    read_buffer: Box<[u8]>,
    write_buffer: VecDeque<u8>,
}

impl<S, const IS_ASYNC: bool> DuplexStream<S, IS_ASYNC> {
    pub fn new(stream: S) -> Self {
        Self::new_with_capacity(stream, 1024)
    }

    pub fn new_with_capacity(stream: S, capacity: usize) -> Self {
        Self {
            stream,
            read_buffer: vec![0; capacity].into(),
            write_buffer: VecDeque::new(),
        }
    }

    pub fn read_buffer(&mut self) -> &[u8] {
        &self.read_buffer
    }

    pub fn write_buffer(&mut self) -> &VecDeque<u8> {
        &self.write_buffer
    }

    pub fn push_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        self.write_buffer.extend(bytes.as_ref());
    }

    fn needs_write(&self) -> bool {
        !self.write_buffer.is_empty()
    }

    fn write_slices(buf: &VecDeque<u8>) -> [IoSlice; 2] {
        let (init, tail) = buf.as_slices();
        [IoSlice::new(init), IoSlice::new(tail)]
    }
}

impl<S, const IS_ASYNC: bool> DuplexStream<S, IS_ASYNC> {
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

pub(crate) fn escape_byte_string<B>(bytes: B) -> String
where
    B: AsRef<[u8]>,
{
    let bytes = bytes.as_ref();

    bytes
        .iter()
        .map(|byte| match byte {
            0x00..=0x08 => format!("\\x{:02x}", byte),
            0x09 => String::from("\\t"),
            0x0A => String::from("\\n"),
            0x0B => format!("\\x{:02x}", byte),
            0x0C => format!("\\x{:02x}", byte),
            0x0D => String::from("\\r"),
            0x0e..=0x1f => format!("\\x{:02x}", byte),
            0x20..=0x21 => format!("{}", *byte as char),
            0x22 => String::from("\\\""),
            0x23..=0x5B => format!("{}", *byte as char),
            0x5C => String::from("\\\\"),
            0x5D..=0x7E => format!("{}", *byte as char),
            0x7f => format!("\\x{:02x}", byte),
            0x80..=0xff => format!("\\x{:02x}", byte),
        })
        .collect::<Vec<String>>()
        .join("")
}
