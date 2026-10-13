use core::ffi::c_int;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::*;

use dstd::io::Error;

use super::{CTRLC_WAITERS, CTRLC_SEMAPHORE};

#[allow(non_camel_case_types)]
//type sighandler_t = unsafe extern "C" fn(c_int);
type sighandler_t = usize;
const SIG_ERR: sighandler_t = -1_isize as usize;
unsafe extern "C" {
    fn signal(signum: c_int, handler: sighandler_t) -> sighandler_t;
}

const SIGINT: c_int = 2;

static PREV_HANDLER: AtomicUsize = AtomicUsize::new(0);

pub fn register() {
    let ret = unsafe { signal(SIGINT, handle_sigint as *const () as usize) };
    debug_assert!(ret != SIG_ERR, "signal() failed: {}", Error::last_os_error());
    PREV_HANDLER.store(ret, Relaxed);
}

pub fn unregister() {
    let prev_handler = PREV_HANDLER.load(Relaxed);
    let ret = unsafe { signal(SIGINT, prev_handler) };
    debug_assert!(ret != SIG_ERR, "signal() failed: {}", Error::last_os_error());
}

unsafe extern "C" fn handle_sigint(_signal: c_int) {
    let waiters = CTRLC_WAITERS.load(Relaxed);
    CTRLC_SEMAPHORE.fetch_add(waiters, Relaxed);
}
