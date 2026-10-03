use core::mem::ManuallyDrop;
use core::task::{Poll, Context};
use core::pin::Pin;

use dstd::io::{Result, ErrorKind, Read, Write};
use dstd::net::{self, SocketAddr, AsRawSocket, RawSocket};

use crate::reactor::Reactor;
use crate::io::{Interest, AsyncRead, AsyncWrite};
use super::WouldBlock;

pub struct TcpListener {
    inner: net::TcpListener,
}

impl TcpListener {
    // TODO: ToSocketAddrs
    pub fn bind(addr: SocketAddr) -> Result<TcpListener> {
        let sock = net::TcpSocket::new_nonblock(addr)?;
        sock.bind(addr)?;
        let inner = sock.listen()?;
        let this = TcpListener { inner };
        Reactor::get().register(this.as_raw_socket());
        Ok(this)
    }

    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        loop {
            let res = self.inner.accept_ex(true);
            match res {
                Ok((stream, addr)) => return Ok((TcpStream::accepted(stream), addr)),
                Err(err) => match err.kind() {
                    ErrorKind::WouldBlock => {
                        // Wait for self to become readable and retry
                        WouldBlock::new(self, Interest::Readable).await;
                    }
                    _ => return Err(err),
                }
            }
        }
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        Reactor::get().deregister(self.inner.as_raw_socket());
    }
}

impl AsRawSocket for TcpListener {
    fn as_raw_socket(&self) -> RawSocket {
        self.inner.as_raw_socket()
    }

    fn into_raw_socket(self) -> RawSocket {
        let this = ManuallyDrop::new(self);
        this.inner.as_raw_socket()
    }

    fn from_raw_socket(socket: RawSocket) -> TcpListener {
        TcpListener { inner: net::TcpListener::from_raw_socket(socket) }
    }
}

pub struct TcpStream {
    inner: net::TcpStream,
}

// TODO read, write

impl TcpStream {
    fn accepted(inner: net::TcpStream) -> TcpStream {
        Reactor::get().register(inner.as_raw_socket());
        TcpStream { inner }
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        let res = self.inner.read(buf);
        match res {
            Ok(nr) => Poll::Ready(Ok(nr)),
            Err(err) => {
                match err.kind() {
                    ErrorKind::WouldBlock => {
                        Reactor::get().modify(self.as_raw_socket(), Interest::Readable, cx.waker());
                        Poll::Pending
                    }
                    _ => Poll::Ready(Err(err)),
                }
            }
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        let res = self.inner.write(buf);
        match res {
            Ok(nw) => Poll::Ready(Ok(nw)),
            Err(err) => {
                match err.kind() {
                    ErrorKind::WouldBlock => {
                        Reactor::get().modify(self.as_raw_socket(), Interest::Writeable, cx.waker());
                        Poll::Pending
                    }
                    _ => Poll::Ready(Err(err)),
                }
            }
        }
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        Reactor::get().deregister(self.inner.as_raw_socket());
    }
}

impl AsRawSocket for TcpStream {
    fn as_raw_socket(&self) -> RawSocket {
        self.inner.as_raw_socket()
    }

    fn into_raw_socket(self) -> RawSocket {
        let this = ManuallyDrop::new(self);
        this.inner.as_raw_socket()
    }

    fn from_raw_socket(socket: RawSocket) -> TcpStream {
        TcpStream { inner: net::TcpStream::from_raw_socket(socket) }
    }
}
