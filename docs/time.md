# timer implementation
Reactor has a list of deadlines (BinaryHeap, reversed order)<br>
It issues a timed wait for the closest deadline, if no deadlines - no timeout<br>
Insert a deadline = push then signal<br>
On wakeup, wake all expired deadlines
