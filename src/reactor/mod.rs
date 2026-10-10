use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::*;
use core::task::Waker;
use core::cmp::Ordering;

extern crate alloc;
use alloc::collections::BinaryHeap;

use dstd::collections::HashMap;
use dstd::net::RawSocket;
use dstd::sync::{Arc, Mutex};
use dstd::time::Instant;

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
    deadlines: Mutex<BinaryHeap<Deadline>>,
}

impl Reactor {
    /// Creates a new reactor
    pub fn new() -> Reactor {
        Reactor {
            poller: Poller::new(),
            registrations: Mutex::new(HashMap::new()),
            pending_wakeup: AtomicBool::new(false),
            deadlines: Mutex::new(BinaryHeap::new()),
        }
    }

    /// Obtains a reference to the current reactor
    pub fn get() -> Arc<Reactor> {
        Runtime::get().reactor()
    }

    /// Polls the reactor, waiting for new events
    pub fn poll(&self) {
        let closest_deadline = self.deadlines.lock().peek().map(|d| d.deadline);

        self.pending_wakeup.store(false, Relaxed);
        self.poller.poll(&self.registrations, closest_deadline);
        self.pending_wakeup.store(true, Relaxed);

        // Empty expired deadlines
        let now = Instant::now();
        // Keep ones that are after now
        self.deadlines.lock().retain(|d| d.deadline > now);
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

    /// Removes a file descriptor. You should call this every time a fd is dropped, otherwise resources would be leaked
    pub fn deregister(&self, fd: RawSocket) {
        self.poller.deregister(fd);
        self.registrations.lock().remove(&fd);
    }

    /// Creates a new timeout: reactor will wake `waker` when it expires
    pub fn push_timeout(&self, deadline: Instant, waker: Waker) {
        let mut deadlines = self.deadlines.lock();
        let do_signal = match deadlines.peek() {
            // Do signal if closest deadline was larger than the new one
            Some(closest) => closest.deadline > deadline,
            // Do signal if there were no deadlines
            None => true,
        };
        deadlines.push(Deadline { deadline, waker });
        drop(deadlines);

        if do_signal {
            // Signal to relaunch epoll_wait with the updated timeout
            self.signal();
        }

        // In practice, that signal rarely fires, because our tasks are polled inside our Executor. There are several ways to trigger it:
        // - Schedule a sleep from a blocking thread (i.e. using spawn inside spawn_blocking)
        // - Schedule a sleep from a different runtime
    }
}

struct Deadline {
    deadline: Instant,
    waker: Waker,
}

impl Drop for Deadline {
    fn drop(&mut self) {
        self.waker.wake_by_ref();
    }
}

impl PartialEq for Deadline {
    fn eq(&self, other: &Deadline) -> bool {
        self.deadline == other.deadline
    }
}

impl Eq for Deadline {}

impl PartialOrd for Deadline {
    fn partial_cmp(&self, other: &Deadline) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Deadline {
    fn cmp(&self, other: &Deadline) -> Ordering {
        self.deadline.cmp(&other.deadline).reverse()
    }
}
