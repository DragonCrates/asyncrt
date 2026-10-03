#![no_std]

use dstd::init::Termination;
use crate::runtime::{Runtime, JoinHandle, ThreadJoinHandle};

pub fn main<F: Future<Output = T> + Send + 'static, T: Termination + Send + 'static>(fut: F) -> T {
    Runtime::new().block_on(fut)
}

pub fn spawn<F: Future<Output = T> + Send + 'static, T: Send + 'static>(f: F) -> JoinHandle<F, T> {
    Runtime::get().spawn(f)
}

pub fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(f: F) -> ThreadJoinHandle<F, T> {
    Runtime::get().spawn_blocking(f)
}

#[macro_export]
macro_rules! main {
    ($name:ident) => {
        mod __asyncrt_private {
            use dstd::init::Termination;
            dstd::main!(main);
            fn main() -> impl Termination {
                $crate::main(super::$name())
            }
        }
    }
}

// TODO list:
// - spawn_blocking
// - timers
// - sync primitives
pub mod io;
pub mod net;
pub mod reactor;
pub mod runtime;

/// Defines a block that can be configured-out entirely
macro_rules! block {
    ($($args:tt)*) => { $($args)* };
}
pub(crate) use block;
