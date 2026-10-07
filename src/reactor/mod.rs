use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::*;
use core::task::Waker;

use dstd::collections::HashMap;
use dstd::net::RawSocket;
use dstd::sync::{Arc, Mutex};

use crate::runtime::Runtime;
use crate::io::Interest;

#[cfg(any(target_os = "linux", target_os = "android"))]
crate::block! {
    mod epoll;
    use epoll::Epoll as Poller;
}

type Registrations = HashMap<RawSocket, (Interest, Waker)>;

pub struct Reactor {
    poller: Poller,
    registrations: Mutex<Registrations>,
    pending_wakeup: AtomicBool,
}

impl Reactor {
    pub fn new() -> Reactor {
        Reactor {
            poller: Poller::new(),
            registrations: Mutex::new(HashMap::new()),
            pending_wakeup: AtomicBool::new(false),
        }
    }

    pub fn get() -> Arc<Reactor> {
        Runtime::get().reactor()
    }

    pub fn poll(&self) {
        self.pending_wakeup.store(false, Relaxed);
        self.poller.poll(&self.registrations);
        self.pending_wakeup.store(true, Relaxed);
    }

    /// Sends a wakeup signal
    pub fn signal(&self) {
        if self.pending_wakeup.load(Relaxed) { return; }
        self.pending_wakeup.store(true, Relaxed);
        self.poller.signal();
    }

    /// Registers a file descriptor. You should call this every time a new fd is created
    pub fn register(&self, fd: RawSocket) {
        self.poller.register(fd);
    }

    /// Rearms a file descriptor. You should call this every time a read or write returns `EWOULDBLOCK`
    pub fn modify(&self, fd: RawSocket, interest: Interest, waker: &Waker) {
        self.registrations.lock().insert(fd, (interest, waker.clone()));
    }

    /// Removes a file descriptor. You should call this every time a fd is dropped
    pub fn deregister(&self, fd: RawSocket) {
        self.poller.deregister(fd);
        self.registrations.lock().remove(&fd);
    }
}
