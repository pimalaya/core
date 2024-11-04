use std::{
    io::{Error, ErrorKind, Read, Result, Write},
    net::{TcpStream, ToSocketAddrs},
};

impl super::TcpStream {
    pub fn std_connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(TcpStream::connect(addr)?.into())
    }
}

impl From<TcpStream> for super::TcpStream {
    fn from(stream: TcpStream) -> Self {
        Self::Std(stream)
    }
}

impl Read for super::TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            Self::Std(stream) => stream.read(buf),
            #[cfg(feature = "async-std")]
            Self::AsyncStd(stream) => {
                futures::executor::block_on(futures::AsyncReadExt::read(stream, buf))
            }
            #[cfg(feature = "tokio")]
            Self::Tokio(stream) => {
                futures::executor::block_on(futures::AsyncReadExt::read(stream, buf))
            }
            _ => Err(Error::new(
                ErrorKind::Unsupported,
                "cannot read from TCP stream",
            )),
        }
    }
}

impl Write for super::TcpStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            Self::Std(stream) => stream.write(buf),
            #[cfg(feature = "async-std")]
            Self::AsyncStd(stream) => {
                futures::executor::block_on(futures::AsyncWriteExt::write(stream, buf))
            }
            #[cfg(feature = "tokio")]
            Self::Tokio(stream) => {
                futures::executor::block_on(futures::AsyncWriteExt::write(stream, buf))
            }
            _ => Err(Error::new(
                ErrorKind::Unsupported,
                "cannot write into TCP stream",
            )),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            Self::Std(stream) => stream.flush(),
            #[cfg(feature = "async-std")]
            Self::AsyncStd(stream) => {
                futures::executor::block_on(futures::AsyncWriteExt::flush(stream))
            }
            #[cfg(feature = "tokio")]
            Self::Tokio(stream) => {
                futures::executor::block_on(futures::AsyncWriteExt::flush(stream))
            }
            _ => Err(Error::new(
                ErrorKind::Unsupported,
                "cannot flush TCP stream",
            )),
        }
    }
}
