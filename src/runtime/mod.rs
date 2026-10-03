use dstd::sync::Arc;

use crate::reactor::Reactor;

mod executor;
use executor::Executor;
mod task;
use task::{Task, DynTask};
pub use task::JoinHandle;
mod guard;
pub use guard::EnterGuard;
mod thread_pool;
use thread_pool::ThreadPool;
pub use thread_pool::ThreadJoinHandle;

pub struct Runtime {
    reactor: Arc<Reactor>,
    executor: Executor,
    thread_pool: ThreadPool,
}

impl Runtime {
    pub fn new() -> Arc<Runtime> {
        Arc::new(Runtime {
            reactor: Arc::new(Reactor::new()),
            executor: Executor::new(),
            thread_pool: ThreadPool::new(),
        })
    }

    pub fn get() -> Arc<Runtime> {
        EnterGuard::get()
    }

    pub fn reactor(&self) -> Arc<Reactor> {
        Arc::clone(&self.reactor)
    }

    pub fn block_on<F: Future<Output = T> + Send + 'static, T: Send + 'static>(self: &Arc<Self>, fut: F) -> T {
        let _guard = self.enter();

        let task = self.spawn(fut);
        loop {
            self.executor.run();
            if task.is_finished() {
                return task.join_blocking();
            }
            self.reactor.poll();
        }
    }

    pub(crate) fn schedule(&self, task: Arc<DynTask>) {
        self.executor.schedule(task);
    }

    pub fn spawn<F: Future<Output = T> + Send + 'static, T: Send + 'static>(self: &Arc<Self>, fut: F) -> JoinHandle<F, T> {
        let task = Task::new(fut, self);
        let task_clone = Arc::clone(&task);
        let dyn_task = task as Arc<DynTask>;
        self.executor.schedule(dyn_task);
        JoinHandle::new(task_clone)
    }

    pub fn spawn_blocking<F: FnOnce() -> T + Send + 'static, T: Send + 'static>(&self, f: F) -> ThreadJoinHandle<F, T> {
        self.thread_pool.spawn(f)
    }

    pub fn enter(self: &Arc<Self>) -> EnterGuard {
        EnterGuard::enter(self.clone())
    }
}
