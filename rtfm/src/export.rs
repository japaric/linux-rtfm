use core::{
    cell::Cell,
    mem,
    ops::Range,
    ptr,
    sync::atomic::{AtomicBool, AtomicI32, Ordering},
};

use heapless::spsc::SingleCore;
pub use heapless::{
    consts,
    i::{BinaryHeap as iBinaryHeap, Queue as iQueue},
    spsc::Queue,
    BinaryHeap,
};
use linux_io::Stderr;
pub use linux_sys::{
    cty::c_void, exit, getpid, pause, pid_t, sched_yield, siginfo_t, timer_t, SI_QUEUE,
};
use linux_sys::{sched_param, sigaction, sigevent, sighandler_t, sigval_t, SIGRTMIN};

pub use crate::tq::{NotReady, TimerQueue};

pub struct Barrier {
    inner: AtomicBool,
}

impl Barrier {
    pub const fn new() -> Self {
        Self {
            inner: AtomicBool::new(false),
        }
    }

    pub fn release(&self) {
        self.inner.store(true, Ordering::Release)
    }

    pub fn wait(&self) {
        while !self.inner.load(Ordering::Acquire) {}
    }
}

pub struct Pid {
    inner: AtomicI32,
}

impl Pid {
    pub const fn uninit() -> Self {
        Self {
            inner: AtomicI32::new(0),
        }
    }

    pub fn get(&self) -> pid_t {
        self.inner.load(Ordering::Relaxed)
    }

    pub fn init(&self, pid: pid_t) {
        self.inner.store(pid, Ordering::Relaxed)
    }

    pub fn wait(&self) -> pid_t {
        loop {
            let pid = self.inner.load(Ordering::Relaxed);

            if pid == 0 {
                linux_sys::sched_yield()
            } else {
                break pid;
            }
        }
    }
}

pub struct Timer {
    inner: AtomicI32,
}

impl Timer {
    pub const fn uninit() -> Self {
        Self {
            inner: AtomicI32::new(0),
        }
    }

    pub fn get(&self) -> timer_t {
        self.inner.load(Ordering::Relaxed)
    }

    pub fn init(&self, timer: timer_t) {
        self.inner.store(timer, Ordering::Relaxed)
    }
}

pub type FreeQueue<N> = Queue<u8, N, u8, SingleCore>;

// The PID `0` represents the current process
const OURSELVES: pid_t = 0;

pub unsafe fn init_runtime(signo_max: Option<u8>) {
    // NOTE all threads spawned (`sys_clone`) from this one will inherit these settings

    // start by running all threads on a single core
    set_affinity(OURSELVES, 0);

    // raise the priority to the minimal real-time priority
    linux_sys::sched_setscheduler(
        OURSELVES,
        linux_sys::SCHED_FIFO,
        &sched_param { sched_priority: 1 },
    )
    .unwrap_or_else(|_| {
        fatal(
            "error: couldn't change scheduling policy; \
             run `sudo setcap cap_sys_nice+ep $binary` first\n",
        )
    });

    // block all the used real-time signals; this is equivalent to `interrupt::disable`
    if let Some(signo) = signo_max {
        let mask = ((1 << (signo + 1)) - 1) << (SIGRTMIN - 1);
        linux_sys::rt_sigprocmask(
            linux_sys::SIG_BLOCK,
            &mask,
            ptr::null_mut(),
        )
        .unwrap_or_else(|_| fatal("error: couldn't change the signal mask\n"));
    }
}

pub unsafe fn spawn(child: extern "C" fn() -> !) -> pid_t {
    const PAGE_SIZE: u64 = 4 * 1024; // 4 KiB (output of `getconf PAGESIZE`)
    const STACK_SIZE: u64 = 2 * 1024 * PAGE_SIZE; // 8 MiB (output of `ulimit -s`)

    linux_sys::mmap(
        0,          // address; 0 means any page-aligned address
        STACK_SIZE, // length of mapping
        linux_sys::PROT_READ | // read access
        linux_sys::PROT_WRITE, // write access
        linux_sys::MAP_ANONYMOUS | // mapping is not backed by any file
        linux_sys::MAP_PRIVATE | // mapping is private to other threads / processes
        linux_sys::MAP_GROWSDOWN | // mapping suitable for stacks
        linux_sys::MAP_UNINITIALIZED, // leave memory uninitialized
        !0,         // file descriptor; needs to be `-1` because of MAP_ANONYMOUS
        0,          // offset; ignored because of MAP_ANONYMOUS
    )
    .and_then(|stack_low| {
        // the stack grows downwards so we must pass the highest address of the mapping to `clone`
        let stack_high = (stack_low as u64 + STACK_SIZE) as *mut _;

        // spin a new thread
        linux_sys::x86_64_clone(
            linux_sys::CLONE_VM | // new thread shares memory with the parent
            linux_sys::CLONE_THREAD | // share thread group
            linux_sys::CLONE_SIGHAND, // shared signal handlers; required by `CLONE_THREAD`
            stack_high,
            child,
        )
    })
    .unwrap_or_else(|_| fatal("error: couldn't spawn a new thread\n"))
}

pub unsafe fn set_affinity(tid: pid_t, core: u8) {
    linux_sys::sched_setaffinity(tid, &[1 << core, 0, 0, 0, 0, 0, 0, 0])
        .unwrap_or_else(|_| fatal("error: couldn't change CPU affinity\n"));
}

pub unsafe fn timer_create(tid: Option<pid_t>, signo: u8) -> timer_t {
    let (sigev_notify, sigev_tid) = if let Some(tid) = tid {
        // multi-core application
        (linux_sys::SIGEV_THREAD_ID, tid)
    } else {
        // single-core application
        (linux_sys::SIGEV_SIGNAL, 0)
    };
    linux_sys::timer_create(
        linux_sys::CLOCK_MONOTONIC,
        &sigevent {
            sigev_value: sigval_t { sival_int: 0 },
            sigev_signo: SIGRTMIN + i32::from(signo),
            sigev_notify,
            sigev_tid,
        },
    )
    .unwrap_or_else(|_| fatal("error: couldn't create a timer\n"))
}

pub unsafe fn lock<T, R>(
    ptr: *mut T,
    priority: &Priority,
    ceiling: u8,
    range: Range<u8>,
    f: impl FnOnce(&mut T) -> R,
) -> R {
    let current = priority.get();

    if current < ceiling {
        priority.set(ceiling);
        mask(range.clone(), current, ceiling, true);
        let r = f(&mut *ptr);
        mask(range, current, ceiling, false);
        priority.set(current);
        r
    } else {
        f(&mut *ptr)
    }
}

pub unsafe fn mask(Range { start, end }: Range<u8>, current: u8, ceiling: u8, block: bool) {
    let len = end.wrapping_sub(start);
    let mask =
        ((1 << (ceiling - current)) - 1) << (SIGRTMIN - 1 + i32::from(start + len - ceiling));
    linux_sys::rt_sigprocmask(
        if block {
            linux_sys::SIG_BLOCK
        } else {
            linux_sys::SIG_UNBLOCK
        },
        &mask,
        ptr::null_mut(),
    )
    .unwrap_or_else(|_| fatal("error: couldn't change the signal mask\n"));
}

pub unsafe fn enqueue(tgid: i32, tid: Option<i32>, signo: u8, task: u8, index: u8) {
    let mut si: siginfo_t = mem::uninitialized();
    si.si_code = linux_sys::SI_QUEUE;
    si.si_value = (usize::from(task) << 8) + usize::from(index);

    if let Some(tid) = tid {
        linux_sys::rt_tgsigqueueinfo(tgid, tid, SIGRTMIN + i32::from(signo), &si)
            .unwrap_or_else(|_| fatal("error: couldn't enqueue signal\n"));
    } else {
        linux_sys::rt_sigqueueinfo(tgid, SIGRTMIN + i32::from(signo), &si)
            .unwrap_or_else(|_| fatal("error: couldn't enqueue signal\n"));
    }
}

pub unsafe fn register(
    Range { start, end }: Range<u8>,
    priority: u8,
    sigaction: extern "C" fn(i32, &mut siginfo_t, *mut c_void),
) {
    extern "C" {
        fn __restorer() -> !;
    }

    let len = end.wrapping_sub(start);
    let mask = (1 << len) - 1;
    linux_sys::rt_sigaction(
        SIGRTMIN + i32::from(end.wrapping_sub(priority)),
        &sigaction {
            sa_: sighandler_t { sigaction },
            sa_flags: linux_sys::SA_RESTORER | linux_sys::SA_SIGINFO,
            sa_restorer: Some(__restorer),
            sa_mask: (mask ^ (mask >> (priority - 1))) << (i32::from(start) + SIGRTMIN - 1),
        },
        ptr::null_mut(),
    )
    .unwrap_or_else(|_| fatal("error: couldn't register signal handler\n"));
}

pub(crate) fn fatal(s: &str) -> ! {
    Stderr.write(s.as_bytes()).ok();
    linux_sys::exit_group(101)
}

// Newtype over `Cell` that forbids mutation through a shared reference
pub struct Priority {
    inner: Cell<u8>,
}

impl Priority {
    #[inline(always)]
    pub unsafe fn new(value: u8) -> Self {
        Priority {
            inner: Cell::new(value),
        }
    }

    // these two methods are used by `lock` (see below) but can't be used from the RTFM application
    #[inline(always)]
    fn set(&self, value: u8) {
        self.inner.set(value)
    }

    #[inline(always)]
    fn get(&self) -> u8 {
        self.inner.get()
    }
}

pub fn assert_send<T>()
where
    T: Send,
{
}

pub fn assert_sync<T>()
where
    T: Sync,
{
}
