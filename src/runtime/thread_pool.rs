use core::task::{Poll, Context, Waker};
use core::pin::Pin;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering::*;

extern crate alloc;
use alloc::borrow::ToOwned;

use dstd::thread;
use dstd::sync::{Arc, Mutex, Condvar, Channel};

use crate::runtime::Runtime;

pub struct ThreadPool {
    nworkers: Arc<AtomicU32>,
    task_channel: Arc<Channel<Arc<DynThreadTask>>>,
}

impl ThreadPool {
    pub fn new() -> ThreadPool {
        ThreadPool {
            nworkers: Arc::new(AtomicU32::new(0)),
            task_channel: Arc::new(Channel::new()),
        }
    }

    pub fn spawn<F, T>(&self, f: F) -> ThreadJoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let task = Arc::new(ThreadTask::new(f));
        let task_clone = Arc::clone(&task);
        if self.nworkers.load(Relaxed) == 0 {
            self.make_thread();
        }
        self.task_channel.send(task_clone as Arc<DynThreadTask>);
        ThreadJoinHandle(task)
    }

    fn make_thread(&self) {
        let nworkers_clone = Arc::clone(&self.nworkers);
        let channel_clone = Arc::clone(&self.task_channel);
        let rt_clone = Runtime::get();
        thread::spawn(move || {
            // TODO: threads should exit after inactivity. Needs recv_timeout
            let _guard = rt_clone.enter();
            loop {
                nworkers_clone.fetch_add(1, Relaxed);
                let task = channel_clone.recv();
                nworkers_clone.fetch_sub(1, Relaxed);
                task.run();
            }
        });
    }
}

struct ThreadTask<F, T> {
    f: Mutex<Option<F>>,
    ret: Mutex<Option<T>>,
    ret_waker: Mutex<Waker>,
    ret_cond: Condvar,
}

impl<F, T> ThreadTask<F, T> {
    fn new(f: F) -> ThreadTask<F, T> {
        ThreadTask {
            f: Mutex::new(Some(f)),
            ret: Mutex::new(None),
            ret_waker: Mutex::new(Waker::noop().clone()),
            ret_cond: Condvar::new(),
        }
    }
}

type DynThreadTask = dyn ThreadTaskTrait;

trait ThreadTaskTrait: Send + Sync {
    fn run(&self);
}

impl<F, T> ThreadTaskTrait for ThreadTask<F, T>
where
    F: FnOnce() -> T + Send,
    T: Send,
{
    fn run(&self) {
        let f = self.f.lock().take().unwrap();
        let ret = f();
        *self.ret.lock() = Some(ret);
        self.ret_waker.lock().wake_by_ref();
        self.ret_cond.notify_one();
    }
}

pub struct ThreadJoinHandle<T>(Arc<dyn ThreadTaskHandle<T>>);

impl<T> ThreadJoinHandle<T> {
    pub fn is_finished(&self) -> bool {
        self.0.is_finished()
    }

    pub fn join_blocking(self) -> T {
        self.0.join_blocking()
    }
}

impl<T> Future for ThreadJoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        self.0.poll(cx)
    }
}

trait ThreadTaskHandle<T>: Send + Sync {
    fn is_finished(&self) -> bool;
    fn join_blocking(&self) -> T;
    fn poll(&self, cx: &mut Context<'_>) -> Poll<T>;
}

impl<F: Send, T: Send> ThreadTaskHandle<T> for ThreadTask<F, T> {
    fn is_finished(&self) -> bool {
        self.ret.lock().is_some()
    }

    fn join_blocking(&self) -> T {
        let mut ret = self.ret.lock();
        loop {
            if let Some(ret) = ret.take() {
                return ret;
            }
            ret = self.ret_cond.wait(ret);
        }
    }

    fn poll(&self, cx: &mut Context<'_>) -> Poll<T> {
        let mut ret = self.ret.lock();
        if let Some(ret) = ret.take() {
            Poll::Ready(ret)
        } else {
            cx.waker().clone_into(&mut *self.ret_waker.lock());
            Poll::Pending
        }
    }
}
