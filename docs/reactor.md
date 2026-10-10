# Reactor operations
Add:
- epoll: `EPOLL_CTL_ADD`
- windows: nothing

Rearm:
- epoll: just update waker and interest
- windows: queue `IOCTL_AFD_POLL` (if operation is not already in flight and interest not changed)

Delete:
- epoll, kqueue: `EPOLL_CTL_DEL` (to be safe with duplicated fds)
- windows: cancel OVERLAPPED `IOCTL_AFD_POLL` and free

ReactorExt on Windows can be used to queue other OVERLAPPED operations manually
On Linux, this is not needed, because everything is already a fd - you can poll on everything
On BSD, similar ReactorExt could be made to add other kqueue operations
