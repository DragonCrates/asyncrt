use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::*;
use core::task::{Context, Waker, Poll};

extern crate alloc;
use alloc::boxed::Box;
use alloc::borrow::ToOwned;
use alloc::task::Wake;

use dstd::sync::{Arc, Weak, Mutex, Condvar};

use super::Runtime;

pub struct Task<T> {
    fut: Mutex<Pin<Box<dyn Future<Output = T> + Send>>>,
    ret: Mutex<Option<T>>,
    ret_waker: Mutex<Waker>,
    ret_cond: Condvar,
    pending_wakeup: AtomicBool,
    runtime: Weak<Runtime>
}

impl<T> Task<T> {
    pub(crate) fn new<F>(fut: F, runtime: &Arc<Runtime>) -> Arc<Task<T>>
    where
        F: Future<Output = T> + Send + 'static
    {
        let task = Task {
            fut: Mutex::new(Box::pin(fut)),
            ret: Mutex::new(None),
            ret_waker: Mutex::new(Waker::noop().clone()),
            ret_cond: Condvar::new(),
            pending_wakeup: AtomicBool::new(false),
            runtime: Arc::downgrade(runtime),
        };
        Arc::new(task)
    }
}

impl<T: Send + 'static> Wake for Task<T> {
    fn wake(self: Arc<Self>) {
        if self.pending_wakeup.load(Relaxed) { return; }
        self.pending_wakeup.store(true, Relaxed);
        if let Some(rt) = self.runtime.upgrade() {
            rt.schedule(self);
            rt.reactor.signal();
        }
    }
}

impl<T: Send + 'static> TaskTrait for Task<T> {
    fn drive(self: Arc<Self>) {
        self.pending_wakeup.store(false, Relaxed);
        let waker = Waker::from(Arc::clone(&self));
        let mut cx = Context::from_waker(&waker);
        let res = self.fut.lock().as_mut().poll(&mut cx);
        if let Poll::Ready(res) = res {
            *self.ret.lock() = Some(res);
            self.ret_waker.lock().wake_by_ref();
            self.ret_cond.notify_one();
        }
    }
}

pub(crate) trait TaskTrait: Send + Sync {
    fn drive(self: Arc<Self>);
}

pub type DynTask = dyn TaskTrait;

pub struct JoinHandle<T>(Arc<Task<T>>);

impl<T> JoinHandle<T> {
    pub(crate) fn new(task: Arc<Task<T>>) -> JoinHandle<T> {
        JoinHandle(task)
    }

    pub fn is_finished(&self) -> bool {
        self.0.ret.lock().is_some()
    }

    pub fn join_blocking(self) -> T {
        let mut ret = self.0.ret.lock();
        loop {
            if let Some(ret) = ret.take() {
                return ret;
            }
            ret = self.0.ret_cond.wait(ret);
        }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut ret = self.0.ret.lock();
        if let Some(ret) = ret.take() {
            Poll::Ready(ret)
        } else {
            cx.waker().clone_into(&mut *self.0.ret_waker.lock());
            Poll::Pending
        }
    }
}
