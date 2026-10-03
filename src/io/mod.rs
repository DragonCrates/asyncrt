use core::task::{Poll, Context};
use core::pin::Pin;

use dstd::io::Result;

#[derive(Debug, Clone, Copy)]
pub enum Interest {
    Readable,
    Writeable,
}

pub trait AsyncRead {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>>;
}

impl<T: AsyncRead + Unpin> AsyncRead for &mut T {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        Pin::new(&mut **self).poll_read(cx, buf)
    }
}

pub trait AsyncReadExt {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Read<'a, Self>;
}

impl<T: AsyncRead> AsyncReadExt for T {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Read<'a, Self> {
        Read { io: self, buf }
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
pub struct Read<'a, T: ?Sized> {
    io: &'a mut T,
    buf: &'a mut [u8],
}

impl<T: AsyncRead + Unpin> Future for Read<'_, T> {
    type Output = Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let this = Pin::into_inner(self);
        Pin::new(&mut this.io).poll_read(cx, this.buf)
    }
}

pub trait AsyncWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>>;
}

impl<T: AsyncWrite + Unpin> AsyncWrite for &mut T {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        Pin::new(&mut **self).poll_write(cx, buf)
    }
}

pub trait AsyncWriteExt: AsyncWrite {
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Write<'a, Self>;
}

impl<T: AsyncWrite> AsyncWriteExt for T {
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Write<'a, Self> {
        Write { io: self, buf }
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
pub struct Write<'a, T: ?Sized> {
    io: &'a mut T,
    buf: &'a [u8],
}

impl<T: AsyncWrite + Unpin> Future for Write<'_, T> {
    type Output = Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let this = Pin::into_inner(self);
        Pin::new(&mut this.io).poll_write(cx, this.buf)
    }
}
