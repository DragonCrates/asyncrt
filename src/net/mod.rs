use core::task::{Poll, Context};
use core::pin::Pin;

use dstd::net::{RawSocket, AsRawSocket};

use crate::io::Interest;
use crate::reactor::Reactor;

mod tcp;
pub use tcp::{TcpListener, TcpStream};

/// This future rearms fd in reactor once and then returns
///
/// Because it returns unconditionally, it should never be used without a saved state (like in `ready!` macro
struct WouldBlock {
    fd: RawSocket,
    fired: bool,
    interest: Interest,
}

impl WouldBlock {
    fn new<T: AsRawSocket>(io: &T, interest: Interest) -> WouldBlock {
        WouldBlock {
            fd: io.as_raw_socket(),
            fired: false,
            interest,
        }
    }
}

impl Future for WouldBlock {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if !self.fired {
            // First call - rearm and sleep
            Reactor::get().modify(self.fd, self.interest, cx.waker());
            self.fired = true;
            Poll::Pending
        } else {
            // Second call - everything is ready
            Poll::Ready(())
        }
    }
}
