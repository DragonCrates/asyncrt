use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering::*;
use core::task::{Waker, Poll, Context};
use core::pin::Pin;

extern crate alloc;
use alloc::vec::Vec;

use dstd::sync::Mutex;

#[cfg(unix)]
crate::block! {
    mod unix;
    use unix as sys;
}

/// Waits for a `Ctrl+C` signal
pub async fn ctrl_c() {
    CtrlC::new().await
}

struct CtrlC {
    registered: bool
}

impl CtrlC {
    fn new() -> CtrlC {
        CtrlC {
            registered: false
        }
    }
}

static CTRLC_WAITERS: AtomicU32 = AtomicU32::new(0);
static CTRLC_SEMAPHORE: AtomicU32 = AtomicU32::new(0);
static CTRLC_WAKERS: Mutex<Vec<Waker>> = Mutex::new(Vec::new());

impl Future for CtrlC {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if !self.registered {
            let waiters = CTRLC_WAITERS.fetch_add(1, Relaxed);
            if waiters == 0 { sys::register(); }
            self.registered = true;
        }

        let mut sem = CTRLC_SEMAPHORE.load(Relaxed);
        loop {
            if sem > 0 {
                match CTRLC_SEMAPHORE.compare_exchange(sem, sem - 1, Relaxed, Relaxed) {
                    Ok(_) => return Poll::Ready(()),
                    Err(new) => sem = new,
                }
            } else {
                let mut found_self = false;
                let mut wakers = CTRLC_WAKERS.lock();
                for waker in wakers.iter() {
                    if waker.will_wake(cx.waker()) {
                        found_self = true;
                        break;
                    }
                }

                if !found_self {
                    wakers.push(cx.waker().clone());
                }

                return Poll::Pending;
            }
        }
    }
}

impl Drop for CtrlC {
    fn drop(&mut self) {
        if !self.registered { return; }
        let waiters = CTRLC_WAITERS.fetch_sub(1, Relaxed);
        if waiters == 1 { sys::unregister(); }
    }
}

pub(crate) fn handle_ctrlc_wakers() {
    if CTRLC_SEMAPHORE.load(Relaxed) > 0 {
        for waker in CTRLC_WAKERS.lock().drain(..) {
            waker.wake();
        }
    }
}
