use core::ptr;
use core::ffi::{c_int, c_uint, c_void};

use dstd::io::{Error, ErrorKind};
use dstd::sync::Mutex;
use dstd::time::Instant;

use crate::io::Interest;
use super::Registrations;

unsafe extern "C" {
    /// Creates a new epoll instance
    fn epoll_create(size: c_int) -> c_int;
    /// Waits for events on the epoll instance referred to by the file descriptor `epfd`
    fn epoll_wait(epfd: c_int, events: *mut epoll_event, n: c_int, timeout: c_int) -> c_int;
    /// Add, modify or remove entries in the interest list of the epoll instance
    fn epoll_ctl(epfd: c_int, op: c_int, fd: c_int, event: *mut epoll_event) -> c_int;

    fn eventfd(initval: c_uint, flags: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
}

const EPOLL_CTL_ADD: c_int = 1;
const EPOLL_CTL_DEL: c_int = 2;
//const EPOLL_CTL_MOD: c_int = 3;

const EPOLLIN: u32 = 0x001;
const EPOLLOUT: u32 = 0x004;

const EPOLLET: u32 = 1 << 31;

const EFD_CLOEXEC: c_int = 1 << 19;
const EFD_NONBLOCK: c_int = 1 << 11;

pub struct Epoll {
    epfd: c_int,
    eventfd: c_int
}

impl Epoll {
    pub fn new() -> Epoll {
        let epfd = unsafe { epoll_create(4096) };
        if epfd == -1 {
            panic!("epoll_create failed: {}", Error::last_os_error());
        }

        let eventfd = unsafe { eventfd(0, EFD_CLOEXEC | EFD_NONBLOCK) };
        if eventfd == -1 {
            panic!("eventfd create failed: {}", Error::last_os_error());
        }

        let mut event = epoll_event::default();
        event.events = EPOLLIN | EPOLLET;
        event.data.fd = eventfd;
        let ret = unsafe { epoll_ctl(epfd, EPOLL_CTL_ADD, eventfd, &mut event) };
        if ret == -1 {
            panic!("eventfd registration failed: {}", Error::last_os_error());
        }

        Epoll { epfd, eventfd }
    }

    pub fn poll(&self, registrations: &Mutex<Registrations>, deadline: Option<Instant>) {
        let mut events = [epoll_event::default(); 2048];
        let timeout = match deadline {
            Some(d) => (d - Instant::now()).as_millis() as i32,
            None => -1,
        };

        let ret = unsafe { epoll_wait(self.epfd, events.as_mut_ptr(), events.len() as c_int, timeout) };
        if ret == -1 {
            let err = Error::last_os_error();
            if err.kind() != ErrorKind::Interrupted {
                panic!("epoll_wait failed: {err}");
            }
            // Interrupted - nothing to do
            return;
        }

        if ret == 0 { return; }

        let registrations = registrations.lock();
        for event in &events[..ret as usize] {
            let fd = unsafe { event.data.fd };
            if fd == self.eventfd {
                // Consume eventfd value
                let mut val: u64 = 0;
                let ret = unsafe { read(self.eventfd, &mut val as *mut _ as *mut c_void, 8) };
                if ret == -1 {
                    panic!("eventfd read failed: {}", Error::last_os_error());
                }
            } else {
                if let Some((i, w)) = registrations.get(&fd) {
                    if (i.to_epoll() & event.events) > 0 {
                        w.wake_by_ref();
                    }
                }
            }
        }
    }

    pub fn register(&self, fd: c_int) {
        let mut event = epoll_event::default();
        event.events = EPOLLIN | EPOLLOUT | EPOLLET;
        event.data.fd = fd;
        let ret = unsafe { epoll_ctl(self.epfd, EPOLL_CTL_ADD, fd, &mut event) };
        if ret == -1 {
            panic!("epoll_ctl add failed: {}", Error::last_os_error());
        }
    }

    pub fn deregister(&self, fd: c_int) {
        let ret = unsafe { epoll_ctl(self.epfd, EPOLL_CTL_DEL, fd, ptr::null_mut()) };
        if ret == -1 {
            panic!("epoll_ctl del failed: {}", Error::last_os_error());
        }
    }

    pub fn signal(&self) {
        let val: u64 = 1;
        let ret = unsafe { write(self.eventfd, &val as *const _ as *const c_void, 8) };
        if ret == -1 {
            panic!("eventfd write failed: {}", Error::last_os_error());
        }
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        unsafe {
            self.deregister(self.eventfd);
            close(self.eventfd);
            close(self.epfd);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct epoll_event {
    events: u32,
    data: epoll_data,
}

impl Default for epoll_event {
    fn default() -> epoll_event {
        epoll_event {
            events: 0,
            data: epoll_data {
                u64: 0,
            },
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
union epoll_data {
    ptr: *mut c_void,
    fd: c_int,
    u32: u32,
    u64: u64,
}

trait InterestExt {
    fn to_epoll(&self) -> u32;    
}

impl InterestExt for Interest {
    fn to_epoll(&self) -> u32 {
        match self {
            Interest::Readable => EPOLLIN,
            Interest::Writeable => EPOLLOUT,
        }
    }
}
