use core::task::{Poll, Context};
use core::pin::Pin;

use dstd::time::{TimeDelta, Instant};

use crate::reactor::Reactor;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Sleep {
    registered: bool,
    deadline: Instant
}

impl Sleep {
    pub fn new(deadline: Instant) -> Sleep {
        Sleep {
            registered: false,
            deadline,
        }
    }
}

impl Future for Sleep {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if !self.registered {
            Reactor::get().push_timeout(self.deadline, cx.waker().clone());
            self.registered = true;
            return Poll::Pending;
        }

        if Instant::now() > self.deadline {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub fn sleep(time: TimeDelta) -> Sleep {
    Sleep::new(Instant::now() + time)
}

pub fn usleep(ms: i64) -> Sleep {
    sleep(TimeDelta::from_millis(ms))
}

pub fn sleep_until(deadline: Instant) -> Sleep {
    Sleep::new(deadline)
}
