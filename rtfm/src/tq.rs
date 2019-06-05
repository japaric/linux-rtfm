use core::{cmp::Ordering, ptr};

use heapless::{binary_heap::Min, ArrayLength, BinaryHeap};
use linux_io::time::Instant;
use linux_sys::{itimerspec, pid_t, timer_t, timespec, SIGRTMIN, TIMER_ABSTIME};

use crate::export::fatal;

pub struct TimerQueue<T, N>(pub BinaryHeap<NotReady<T>, N, Min>)
where
    T: Copy,
    N: ArrayLength<NotReady<T>>;

impl<T, N> TimerQueue<T, N>
where
    T: Copy,
    N: ArrayLength<NotReady<T>>,
{
    pub unsafe fn enqueue_unchecked(
        &mut self,
        nr: NotReady<T>,
        tgid_tid: Option<(pid_t, pid_t)>,
        signo: u8,
    ) {
        if self
            .0
            .peek()
            .map(|head| nr.instant < head.instant)
            .unwrap_or(true)
        {
            // new entry has earlier deadline; signal the timer queue
            if let Some((tgid, tid)) = tgid_tid {
                // multi-core application
                linux_sys::tgkill(tgid, tid, SIGRTMIN + i32::from(signo))
                    .unwrap_or_else(|_| fatal("error: couldn't send a signal\n"));
            } else {
                // single core application
                linux_sys::kill(0, SIGRTMIN + i32::from(signo))
                    .unwrap_or_else(|_| fatal("error: couldn't send a signal\n"));
            }
        }

        self.0.push_unchecked(nr);
    }

    pub fn dequeue(&mut self, timer_id: timer_t) -> Option<(T, u8)> {
        if let Some(instant) = self.0.peek().map(|p| p.instant) {
            let now = Instant::now();
            if now >= instant {
                // task became ready
                let nr = unsafe { self.0.pop_unchecked() };

                Some((nr.task, nr.index))
            } else {
                // set a new timeout
                linux_sys::timer_settime(
                    timer_id,
                    TIMER_ABSTIME,
                    &itimerspec {
                        it_interval: timespec {
                            tv_sec: 0,
                            tv_nsec: 0,
                        },
                        it_value: instant.into(),
                    },
                    ptr::null_mut(),
                )
                .unwrap_or_else(|_| fatal("error: couldn't set timeout\n"));

                None
            }
        } else {
            // the queue is empty
            None
        }
    }
}

pub struct NotReady<T>
where
    T: Copy,
{
    pub index: u8,
    pub instant: Instant,
    pub task: T,
}

impl<T> Eq for NotReady<T> where T: Copy {}

impl<T> Ord for NotReady<T>
where
    T: Copy,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.instant.cmp(&other.instant)
    }
}

impl<T> PartialEq for NotReady<T>
where
    T: Copy,
{
    fn eq(&self, other: &Self) -> bool {
        self.instant == other.instant
    }
}

impl<T> PartialOrd for NotReady<T>
where
    T: Copy,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}
