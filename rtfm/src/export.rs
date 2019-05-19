use core::{cell::Cell, mem, ptr};

use heapless::spsc::SingleCore;
pub use heapless::{consts, i::Queue as iQueue, spsc::Queue};
use linux_io::Stderr;
pub use linux_sys::{cty::c_void, exit, getpid, pause, siginfo_t};
use linux_sys::{pid_t, sched_param, sigaction, sighandler_t, sigset_t, SIGRTMIN};

pub type FreeQueue<N> = Queue<u8, N, u8, SingleCore>;
pub type ReadyQueue<T, N> = Queue<(T, u8), N, u8, SingleCore>;

/// Maximum supported priority
// there are 32 real time signals but let's use a lower number
pub const PRIORITY_MAX: u8 = 16;

pub const PRIORITY_MIN: u8 = 1;

// The PID `0` represents the current process
const OURSELVES: pid_t = 0;

pub unsafe fn init_scheduler() {
    // run all threads on a single core
    // NOTE all threads spawned (`sys_clone`) from this one will inherit this core affinity
    linux_sys::sched_setaffinity(OURSELVES, &[1, 0, 0, 0, 0, 0, 0, 0])
        .unwrap_or_else(|_| fatal("error: couldn't change CPU affinity\n"));

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

    // change the priority to the max value -- equivalent do `interrupt::disable()`
    set_priority(PRIORITY_MAX);
}

pub unsafe fn lock<T, R>(
    ptr: *mut T,
    priority: &Priority,
    ceiling: u8,
    f: impl FnOnce(&mut T) -> R,
) -> R {
    let current = priority.get();

    if current < ceiling {
        priority.set(ceiling);
        set_priority(ceiling);
        let r = f(&mut *ptr);
        set_priority(current);
        priority.set(current);
        r
    } else {
        f(&mut *ptr)
    }
}

pub unsafe fn set_priority(priority: u8) {
    // NOTE signal numbers, like `SIGRTMIN`, start at `1` but in signal sets they start at `0`
    // We'll use signal `SIGRTMIN` for the highest priority `PRIORITY_MAX`
    // Higher numbered signals (`SIGRTMIN+1`) will be used for lower priorities (`PRIORITY_MAX-1`)

    linux_sys::rt_sigprocmask(linux_sys::SIG_SETMASK, &mask(priority), ptr::null_mut())
        .unwrap_or_else(|_| fatal("error: couldn't change signal mask\n"));
}

pub unsafe fn enqueue(pid: i32, priority: u8, task: u8, index: u8) {
    let mut si: siginfo_t = mem::uninitialized();
    si.si_code = linux_sys::SI_QUEUE;
    si.si_value = (usize::from(task) << 8) + usize::from(index);
    linux_sys::rt_sigqueueinfo(pid, SIGRTMIN + i32::from(PRIORITY_MAX - priority), &si)
        .unwrap_or_else(|_| fatal("error: couldn't queue signal\n"));
}

pub unsafe fn register(priority: u8, sigaction: extern "C" fn(i32, &mut siginfo_t, *mut c_void)) {
    extern "C" {
        fn __restorer() -> !;
    }

    linux_sys::rt_sigaction(
        SIGRTMIN + i32::from(PRIORITY_MAX - priority),
        &sigaction {
            sa_: sighandler_t { sigaction },
            sa_flags: linux_sys::SA_RESTORER | linux_sys::SA_SIGINFO,
            sa_restorer: Some(__restorer),
            sa_mask: mask(priority),
        },
        ptr::null_mut(),
    )
    .unwrap_or_else(|_| fatal("error: couldn't queue signal\n"));
}

fn mask(priority: u8) -> sigset_t {
    (u64::max_value().wrapping_shl(u32::from(PRIORITY_MAX - priority)) & ((1 << PRIORITY_MAX) - 1))
        << (SIGRTMIN - 1)
}

fn fatal(s: &str) -> ! {
    unsafe {
        Stderr::borrow_unchecked(|stderr| {
            stderr.write_all(s.as_bytes()).ok();
        });

        linux_sys::exit_group(101)
    }
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

pub struct MaybeUninit<T> {
    inner: core::mem::MaybeUninit<T>,
}

impl<T> MaybeUninit<T> {
    pub const fn uninit() -> Self {
        MaybeUninit {
            inner: core::mem::MaybeUninit::uninit(),
        }
    }

    pub fn as_ptr(&self) -> *const T {
        self.inner.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.inner.as_mut_ptr()
    }

    pub unsafe fn read(&self) -> T {
        self.inner.read()
    }

    pub fn write(&mut self, value: T) -> &mut T {
        self.inner.write(value)
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
