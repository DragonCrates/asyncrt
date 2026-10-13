# Control+C handler
ctrlc is a semaphore
register - increment WAITERS, register SIGINT handler if it was 0
receive event:
- signal handler: add WAITERS to SEMAPHORE, signal reactor
- reactor: wake waiters
- waiter: decrement SEMAPHORE
drop - decrement WAITERS, unregister SIGINT if it was 1
