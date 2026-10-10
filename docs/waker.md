# Waker
Waker can be of 2 types:
- Schedule the task to be executed in next run
- Run provided callback

The one you choose is based on what do you want from your executor/runtime

This DeepSeek answer explains it well:

### Should it be?

Polling directly inside wake() has real trade-offs. It's not a "less correct" executor; it's a different one with a different cost profile.

### Reasons it's fine / good:

- No run queue, no scheduler, no allocation per wake. Great for no_std / embedded.
- Lower latency: the event that fires the waker runs the continuation immediately.
- Simpler to implement correctly if you're careful.

### Reasons to not do it (why most general-purpose runtimes enqueue instead):

1. Re-entrancy / unbounded stack growth. If wake() polls, and that poll synchronously completes another future which wakes another task which polls… you recurse. A chain of ready tasks can blow the stack. Enqueueing breaks the recursion into a loop in the runtime.
2. Wake amplification. A future can wake itself, or be woken by many sources. Inline polling turns N wakes into N polls. A run queue lets the runtime coalesce (poll once, or just note "already queued").
3. Fairness / starvation. Inline wake = the waker's caller pays for the polled task. A chatty task can starve siblings. A scheduler can round-robin; inline can't.
4. Work stealing / multi-threaded execution. If wake() polls on the waking thread, the task runs wherever the event came from — maybe a signal handler, an interrupt, an io_uring CQE thread. You often want to move the task to a worker. Enqueue decouples "event happened" from "run here."
5. Cancellation and dropping. Polling inside wake() while holding the task's own lock (the typical Arc<Task> + Mutex<Future> shape) is a deadlock magnet. Enqueueing sidesteps it: the waker just pushes to a queue and returns.
6. Wake vs Wake by Arc. wake() can run on a thread with different permissions, in a context where you can't allocate, can't block, or where running arbitrary user code is dangerous. Inline poll runs arbitrary user code in that context.
