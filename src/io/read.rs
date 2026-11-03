use core::task::{ready, Poll, Context};
use core::pin::Pin;

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
    // TODO: read_to_end
    fn take(self, limit: u64) -> Take<Self> where Self: Sized;
}

impl<T: AsyncRead> AsyncReadExt for T {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Read<'a, Self> {
        Read { io: self, buf }
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

impl<T: AsyncRead + Unpin> Future for Read<'_, T> {
    type Output = Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let this = Pin::into_inner(self);
        Pin::new(&mut this.io).poll_read(cx, this.buf)
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
        let limit = u64_to_usize(*this.limit);
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
