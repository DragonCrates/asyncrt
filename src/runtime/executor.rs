use dstd::sync::{Arc, Channel};

use super::DynTask;

#[derive(Default)]
pub struct Executor {
    queue: Channel<Arc<DynTask>>,
}

impl Executor {
    pub fn new() -> Executor {
        Executor {
            queue: Channel::new(),
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
