use std::io::Result;

use tokio::net::{TcpStream, ToSocketAddrs};

impl super::TcpStream {
    pub async fn tokio_connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(TcpStream::connect(addr).await?.into())
    }
}

impl From<TcpStream> for super::TcpStream {
    fn from(stream: TcpStream) -> Self {
        use tokio_util::compat::TokioAsyncReadCompatExt;
        Self::Tokio(stream.compat())
    }
}
