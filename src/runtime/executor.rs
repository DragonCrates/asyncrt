extern crate alloc;
use alloc::sync::Arc;
use alloc::collections::VecDeque;

use dstd::sync::Mutex;

use super::DynTask;

pub struct Executor {
    queue: MiniChannel<Arc<DynTask>>
}

impl Executor {
    pub fn new() -> Executor {
        Executor {
            queue: MiniChannel::new(),
        }
    }

    pub fn schedule(&self, task: Arc<DynTask>) {
        self.queue.send(task);
    }

    pub fn run(&self) {
        while let Some(task) = self.queue.try_recv() {
            task.drive();
        }
    }
}

struct MiniChannel<T> {
    queue: Mutex<VecDeque<T>>
}

impl<T> MiniChannel<T> {
    fn new() -> MiniChannel<T> {
        MiniChannel {
            queue: Mutex::new(VecDeque::new())
        }
    }

    fn send(&self, item: T) {
        self.queue.lock().push_back(item);
    }

    fn try_recv(&self) -> Option<T> {
        self.queue.lock().pop_front()
    }
}
