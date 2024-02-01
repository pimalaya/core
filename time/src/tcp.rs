use tokio::{
    io::{self, BufReader, ReadHalf, WriteHalf},
    net::TcpStream,
};

pub struct TcpHandler {
    pub reader: BufReader<ReadHalf<TcpStream>>,
    pub writer: WriteHalf<TcpStream>,
}

impl From<TcpStream> for TcpHandler {
    fn from(stream: TcpStream) -> Self {
        let (reader, writer) = io::split(stream);
        let reader = BufReader::new(reader);
        Self { reader, writer }
    }
}
