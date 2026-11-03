use core::task::{ready, Poll, Context};
use core::pin::Pin;

use dstd::io::{ErrorKind, Result};

pub trait AsyncWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;
}

impl<T: AsyncWrite + Unpin + ?Sized> AsyncWrite for &mut T {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        Pin::new(&mut **self).poll_write(cx, buf)
    }
}

pub trait AsyncWriteExt: AsyncWrite {
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Write<'a, Self>;
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAll<'a, Self>;
}

impl<T: AsyncWrite + ?Sized> AsyncWriteExt for T {
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Write<'a, Self> {
        Write { io: self, buf }
    }

    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAll<'a, Self> {
        WriteAll { io: self, buf }
    }
}

#[allow(clippy::module_inception)]
mod write {
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    #[derive(Debug)]
    pub struct Write<'a, T: ?Sized> {
        pub(crate) io: &'a mut T,
        pub(crate) buf: &'a [u8],
    }
}
use write::Write;

impl<T: AsyncWrite + Unpin + ?Sized> Future for Write<'_, T> {
    type Output = Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let this = Pin::into_inner(self);
        Pin::new(&mut this.io).poll_write(cx, this.buf)
    }
}

mod write_all {
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    #[derive(Debug)]
    pub struct WriteAll<'a, T: ?Sized> {
        pub(crate) io: &'a mut T,
        pub(crate) buf: &'a [u8],
    }
}
use write_all::WriteAll;

impl<T: AsyncWrite + Unpin + ?Sized> Future for WriteAll<'_, T> {
    type Output = Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let this = Pin::into_inner(self);
        loop {
            if this.buf.is_empty() { return Poll::Ready(Ok(())); }
            let nw = ready!(Pin::new(&mut this.io).poll_write(cx, this.buf))?;
            if nw == 0 { return Poll::Ready(Err(ErrorKind::WriteZero.into())); }
            this.buf = &this.buf[nw..];
        }
    }
}
