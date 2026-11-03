#[derive(Debug, Clone, Copy)]
pub enum Interest {
    Readable,
    Writeable,
}

mod read;
pub use read::{AsyncRead, AsyncReadExt};
mod buf_read;
pub use buf_read::{AsyncBufRead, AsyncBufReadExt, BufReader};
mod write;
pub use write::{AsyncWrite, AsyncWriteExt};

mod buf;
