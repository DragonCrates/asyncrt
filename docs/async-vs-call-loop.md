# Async or call loop?
Inspired by this article: <https://notgull.net/calloop/>

Async and call loop are different ways to accomplish the same thing: run something when event arrives. However, they have different usecases:
- Async is used mainly on networking I/O. Usually, can be independently polled from multiple threads
- Call loop is used mainly for GUIs. Usually, it is a condvar and mutex that are waited from one thread

To send a custom event, async requires a guaranteed roundtrip to kernel. To receive networking events, call loop requires a separate poller thread. More syscalls, more threads - more overhead

However, most call loops on Linux are implemented using epoll, because events come from a unix socket. On the other hand, on Windows and OSX, there are OS provided call loops, and you can't receive events any other way

So quick TL;DR: use whatever is more natural. GUI with async operations? Integrate your runtime into a call loop. Your call loop is already an epoll? Great. No GUI? Then call loop is probably not required at all
