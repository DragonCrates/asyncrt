use core::task::{ready, Poll, Context};
use core::pin::Pin;
use core::mem;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use dstd::io::Result;

use super::{AsyncRead, AsyncWrite};
use super::buf::Buf;

pub trait AsyncBufRead: AsyncRead {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8]>>;
    fn consume(self: Pin<&mut Self>, amt: usize);
}

impl<T: AsyncBufRead + Unpin + ?Sized> AsyncBufRead for &mut T {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
        Pin::new(&mut **self.get_mut()).poll_fill_buf(cx)
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        Pin::new(&mut **self).consume(amt);
    }
}

pub trait AsyncBufReadExt: AsyncBufRead {
    fn fill_buf(&mut self) -> FillBuf<'_, Self>;
    fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut Vec<u8>) -> ReadUntil<'a, Self>;
    fn read_line<'a>(&'a mut self, buf: &'a mut String) -> ReadLine<'a, Self>;
}

impl<T: AsyncBufRead + ?Sized> AsyncBufReadExt for T {
    fn fill_buf(&mut self) -> FillBuf<'_, Self> {
        FillBuf { io: Some(self) }
    }

    fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut Vec<u8>) -> ReadUntil<'a, Self> {
        ReadUntil { io: self, byte, buf, nw: 0 }
    }

    fn read_line<'a>(&'a mut self, buf: &'a mut String) -> ReadLine<'a, Self> {
        ReadLine { io: self, buf, nw: 0 }
    }
}

mod fill_buf {
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct FillBuf<'a, T: ?Sized> {
        pub(crate) io: Option<&'a mut T>,
    }
}
use fill_buf::FillBuf;

impl<'a, T: AsyncBufRead + Unpin + ?Sized> Future for FillBuf<'a, T> {
    type Output = Result<&'a [u8]>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&'a [u8]>> {
        let this = Pin::into_inner(self);
        let io = this.io.take().expect("polled after completion");
        match Pin::new(&mut *io).poll_fill_buf(cx) {
            Poll::Ready(Ok(buf)) => {
                // What the fuck
                // Anyway, here you go: https://github.com/tokio-rs/tokio/blob/e0f829f033008a86f70402b7ec4ea7f7f4652874/tokio/src/io/util/fill_buf.rs#L40
                let buf = unsafe { mem::transmute::<&[u8], &'a [u8]>(buf) };
                Poll::Ready(Ok(buf))
            }

            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => {
                this.io = Some(io);
                Poll::Pending
            }
        }
    }
}

mod read_until {
    use super::Vec;

    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct ReadUntil<'a, T: ?Sized> {
        pub(crate) io: &'a mut T,
        pub(crate) byte: u8,
        pub(crate) buf: &'a mut Vec<u8>,
        pub(crate) nw: usize,
    }
}
use read_until::ReadUntil;

fn poll_read_until<T: AsyncBufRead + ?Sized>(mut io: Pin<&mut T>, byte: u8, this_buf: &mut Vec<u8>, nw: &mut usize, cx: &mut Context<'_>) -> Poll<Result<usize>> {
    loop {
        let buf = ready!(io.as_mut().poll_fill_buf(cx))?;

        if buf.is_empty() {
            return Poll::Ready(Ok(*nw));
        }

        // Try to find byte
        if let Some(pos) = buf.iter().position(|&i| i == byte) {
            this_buf.extend(&buf[..pos+1]);
            *nw += pos+1;
            io.consume(pos+1);
            return Poll::Ready(Ok(*nw));
        } else {
            this_buf.extend(buf);
            let len = buf.len();
            *nw += len;
            io.as_mut().consume(len);
        }
    }
}

impl<T: AsyncBufRead + Unpin + ?Sized> Future for ReadUntil<'_, T> {
    type Output = Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let this = Pin::into_inner(self);
        let io = Pin::new(&mut *this.io);
        let byte = this.byte;
        let buf = &mut this.buf;
        let nw = &mut this.nw;
        poll_read_until(io, byte, buf, nw, cx)
    }
}

mod read_line {
    use super::String;

    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct ReadLine<'a, T: ?Sized> {
        pub(crate) io: &'a mut T,
        pub(crate) buf: &'a mut String,
        pub(crate) nw: usize,
    }
}
use read_line::ReadLine;

impl<T: AsyncBufRead + Unpin + ?Sized> Future for ReadLine<'_, T> {
    type Output = Result<usize>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<usize>> {
        let this = Pin::into_inner(self);

        let safe_utf8 = this.buf.len();
        let mut bytes_vec = mem::take(this.buf).into_bytes();

        // Now it is basically read_until
        let io = Pin::new(&mut *this.io);
        let ret = poll_read_until(io, b'\n', &mut bytes_vec, &mut this.nw, cx);

        match str::from_utf8(&bytes_vec[safe_utf8..]) {
            Ok(_) => {
                // SAFETY: we just verified that newly read bytes are valid unicode
                *this.buf = unsafe { String::from_utf8_unchecked(bytes_vec) };
                ret
            }

            Err(err) => {
                bytes_vec.truncate(safe_utf8);
                // SAFETY: we just discarded invalid bytes, restoring previous known-valid state
                *this.buf = unsafe { String::from_utf8_unchecked(bytes_vec) };
                Poll::Ready(Err(err.into()))
            }
        }
    }
}

pin_project_lite::pin_project! {
    pub struct BufReader<T> {
        #[pin]
        io: T,
        buf: Buf,
    }
}

impl<T> BufReader<T> {
    pub fn new(io: T) -> BufReader<T> {
        BufReader {
            io,
            buf: Buf::new(),
        }
    }
}

impl<T: AsyncRead> AsyncRead for BufReader<T> {
    #[allow(clippy::len_zero)]
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
        let this = self.as_mut().project();
        let mut nw = 0;

        // Empty the buffer, if it had something
        if this.buf.len() > 0 {
            if buf.len() <= this.buf.len() {
                // buf is smaller than internal buffer
                buf.copy_from_slice(&this.buf[..buf.len()]);
                this.buf.consume(buf.len());
                return Poll::Ready(Ok(buf.len()));
            } else {
                // empty internal buffer
                nw = this.buf.len();
                buf[..nw].copy_from_slice(&this.buf);
                this.buf.consume(nw);
            }
        }

        // If the `len - nw` is more than half of our buffer, just read directly
        if buf.len() - nw > this.buf.capacity() / 2 {
            nw += ready!(this.io.poll_read(cx, &mut buf[nw..]))?;
            return Poll::Ready(Ok(nw));
        }

        // Otherwise, fill the buffer again and copy from it
        ready!(self.as_mut().poll_fill_buf(cx))?;
        // Build pin projection again
        let this = self.project();

        if this.buf.is_empty() {
            return Poll::Ready(Ok(nw));
        }

        let buf_space = buf.len() - nw;
        let to_write = buf_space.min(this.buf.len());
        buf[nw..nw+to_write].copy_from_slice(&this.buf[..to_write]);
        this.buf.consume(to_write);
        Poll::Ready(Ok(nw + to_write))
    }
}

impl<T: AsyncRead> AsyncBufRead for BufReader<T> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
        let this = self.project();

        if this.buf.is_empty() {
            let nr = ready!(this.io.poll_read(cx, this.buf.get_fill()))?;
            this.buf.extend(nr);
        }

        Poll::Ready(Ok(&*this.buf))
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().buf.consume(amt);
    }
}

impl<T: AsyncWrite> AsyncWrite for BufReader<T> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
        self.project().io.poll_write(cx, buf)
    }
}
