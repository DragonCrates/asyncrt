use core::task::{ready, Poll, Context};
use core::pin::Pin;

extern crate alloc;
use alloc::vec::Vec;

use dstd::io::Result;

use pin_project_lite::pin_project;

use super::AsyncBufRead;

pub trait AsyncRead {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>>;
}

impl<T: AsyncRead + Unpin + ?Sized> AsyncRead for &mut T {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        Pin::new(&mut **self).poll_read(cx, buf)
    }
}

pub trait AsyncReadExt {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Read<'a, Self>;
    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> ReadToEnd<'a, Self>;
    fn take(self, limit: u64) -> Take<Self> where Self: Sized;
}

impl<T: AsyncRead> AsyncReadExt for T {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Read<'a, Self> {
        Read { io: self, buf }
    }

    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> ReadToEnd<'a, Self> {
        ReadToEnd::new(self, buf)
    }

    fn take(self, limit: u64) -> Take<Self> {
        Take { io: self, limit }
    }
}

#[allow(clippy::module_inception)]
mod read {
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    #[derive(Debug)]
    pub struct Read<'a, T: ?Sized> {
        pub(crate) io: &'a mut T,
        pub(crate) buf: &'a mut [u8],
    }
}
use read::Read;

impl<T: AsyncRead + Unpin + ?Sized> Future for Read<'_, T> {
    type Output = Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let this = Pin::into_inner(self);
        Pin::new(&mut this.io).poll_read(cx, this.buf)
    }
}

mod read_to_end {
    use super::Vec;

    #[must_use = "futures do nothing unless you `.await` or poll them"]
    #[derive(Debug)]
    pub struct ReadToEnd<'a, T: ?Sized> {
        pub(crate) io: &'a mut T,
        pub(crate) buf: &'a mut Vec<u8>,
        pub(crate) init: usize,
    }
}
use read_to_end::ReadToEnd;

impl<'a, T> ReadToEnd<'a, T> {
    fn new(io: &'a mut T, buf: &'a mut Vec<u8>) -> ReadToEnd<'a, T> {
        let init = buf.len();
        ReadToEnd { io, buf, init }
    }
}

impl<T: AsyncRead + Unpin + ?Sized> Future for ReadToEnd<'_, T> {
    type Output = Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let this = Pin::into_inner(self);

        if this.buf.capacity() < 128 { this.buf.reserve(128 - this.buf.len()); }
        this.buf.resize(this.buf.capacity(), 0);

        loop {
            if this.init + this.buf.len()/8 >= this.buf.len() {
                this.buf.resize(this.buf.capacity()*2, 0);
            }

            let nr = ready!(Pin::new(&mut this.io).poll_read(cx, &mut this.buf[this.init..]))?;
            if nr == 0 { return Poll::Ready(Ok(())); }
            this.init += nr;
        }
    }
}

impl<T: ?Sized> Drop for ReadToEnd<'_, T> {
    fn drop(&mut self) {
        self.buf.truncate(self.init);
    }
}

pin_project! {
    pub struct Take<T> {
        #[pin]
        io: T,
        limit: u64,
    }
}

impl<T> Take<T> {
    pub fn limit(&self) -> u64 {
        self.limit
    }
}

fn u64_to_usize(n: u64) -> usize {
    n.try_into().unwrap_or(usize::MAX)
}

impl<T: AsyncRead> AsyncRead for Take<T> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        // Don't call the reader if limit iss exhausted
        if self.limit == 0 { return Poll::Ready(Ok(0)); }

        let this = self.project();
        let limit = u64_to_usize(*this.limit).min(buf.len());
        let nr = ready!(this.io.poll_read(cx, &mut buf[..limit]))?;
        *this.limit -= nr as u64;
        Poll::Ready(Ok(nr))
    }
}

impl<T: AsyncBufRead> AsyncBufRead for Take<T> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
        self.project().io.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().io.consume(amt);
    }
}
