use core::ops::{Deref, DerefMut};

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec;

pub struct Buf {
    buf: Box<[u8; 4096]>,
    start: usize,
    end: usize,
}

impl Buf {
    /// Creates a new buf
    pub fn new() -> Buf {
        let buf = vec![0; 4096].into_boxed_slice().try_into().unwrap();
        Buf {
            buf,
            start: 0,
            end: 0,
        }
    }

    /// Returns the space after the end of the buffer, to fill it with new data
    pub fn get_fill(&mut self) -> &mut [u8] {
        &mut self.buf[self.end..]
    }

    /// Extends the buffer with newly written data
    pub fn extend(&mut self, amt: usize) {
        self.end += amt;
        debug_assert!(self.end <= self.buf.len(), "attempt to extend buffer beyond its end");
    }

    fn reset(&mut self) {
        self.start = 0;
        self.end = 0;
    }

    /// Consumes `amt` bytes from the buffer
    pub fn consume(&mut self, amt: usize) {
        self.start += amt;
        debug_assert!(self.start <= self.end, "tried to consume more than buffer len");
        if self.start == self.end { self.reset(); }
    }

    /// Returns the maximum possible capacity for this buffer
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }
}

impl Deref for Buf {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.buf[self.start..self.end]
    }
}

impl DerefMut for Buf {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.buf[self.start..self.end]
    }
}
