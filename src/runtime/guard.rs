use core::cell::{Cell, RefCell};
use core::marker::PhantomData;
use dstd::sync::Arc;

use crate::runtime::Runtime;

pub struct EnterGuard {
    prev: Option<Arc<Runtime>>,
    depth: usize,
    /// Make it !Send because it references thread local data
    marker: PhantomData<*mut u8>,
}

struct RuntimeCell {
    runtime: RefCell<Option<Arc<Runtime>>>,
    depth: Cell<usize>,
}

dstd::thread_local! {
    static RUNTIME: RuntimeCell = RuntimeCell {
        runtime: RefCell::new(None),
        depth: Cell::new(0),
    }
}

impl EnterGuard {
    fn new(prev: Option<Arc<Runtime>>, depth: usize) -> EnterGuard {
        EnterGuard {
            prev,
            depth,
            marker: PhantomData,
        }
    }

    pub(crate) fn get() -> Arc<Runtime> {
        RUNTIME.with(|runtime_cell| {
            let mut borrow = runtime_cell.runtime.borrow_mut();
            let rt = borrow.as_mut().expect("called an asyncrt function without an active runtime");
            Arc::clone(rt)
        })
    }

    pub fn enter(runtime: Arc<Runtime>) -> EnterGuard {
        RUNTIME.with(|runtime_cell| {
            let prev = runtime_cell.runtime.take();
            let depth = runtime_cell.depth.get() + 1;
            *runtime_cell.runtime.borrow_mut() = Some(runtime);
            runtime_cell.depth.set(depth);
            EnterGuard::new(prev, depth)
        })
    }
}

impl Drop for EnterGuard {
    fn drop(&mut self) {
        RUNTIME.with(|runtime_cell| {
            if self.depth != runtime_cell.depth.get() {
                panic!("EnterGuard should not be dropped out of order");
            }
            *runtime_cell.runtime.borrow_mut() = self.prev.take();
            runtime_cell.depth.set(self.depth - 1);
        })
    }
}
