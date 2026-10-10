# timer implementation
Reactor has a list of deadlines (BinaryHeap, reversed order)
It issues a timed wait for the closest deadline, if no deadlines - no timeout
Insert a deadline = push then signal
On wakeup, wake all expired deadlines
