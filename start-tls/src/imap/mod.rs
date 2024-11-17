#[cfg(feature = "async")]
pub mod futures;
#[cfg(feature = "blocking")]
pub mod std;

#[derive(Clone, Debug)]
pub struct ImapStartTls {
    read_buffer: Vec<u8>,
    handshake_discarded: bool,
}

impl ImapStartTls {
    const COMMAND: &'static str = "A1 STARTTLS\r\n";

    pub fn new() -> Self {
        Self {
            read_buffer: vec![0; 1024],
            handshake_discarded: false,
        }
    }

    pub fn set_read_buffer_capacity(&mut self, capacity: usize) {
        self.read_buffer = vec![0; capacity];
    }

    pub fn with_read_buffer_capacity(mut self, capacity: usize) -> Self {
        self.set_read_buffer_capacity(capacity);
        self
    }

    pub fn set_handshake_discarded(&mut self, discarded: bool) {
        self.handshake_discarded = discarded;
    }

    pub fn with_handshake_discarded(mut self, discarded: bool) -> Self {
        self.set_handshake_discarded(discarded);
        self
    }
}

impl Default for ImapStartTls {
    fn default() -> Self {
        Self::new()
    }
}
