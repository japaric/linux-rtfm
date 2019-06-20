//! Linux system call interface

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(global_asm)]
#![feature(proc_macro_hygiene)]
#![no_std]

#[macro_use]
mod macros;
mod consts;
mod types;

use core::{
    hint,
    mem::{self, MaybeUninit},
    ptr,
};

pub use cty;
use cty::{c_char, c_int, c_uint, c_ulong, c_void, size_t};
use sc::syscall;
use ufmt::uwrite;

pub use consts::*;
pub use types::*;

// System calls ordered by their (x86_64) "number"

// NR = 1
/// Write to a file descriptor
///
/// See `man 2 write` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/fs/read_write.c#L607
///
/// C signature: `ssize_t write(c_uint fd, const char *buf, size_t count);`
pub unsafe fn write(fd: c_uint, buf: &[u8]) -> Result<usize, Error> {
    let count: size_t = buf.len();
    let buf = buf.as_ptr() as *const c_char;
    check!(syscall!(WRITE, fd, buf, count)).map(|ret| ret as usize)
}

// NR = 9
/// Map files or devices into memory
///
/// See `man 2 mmap` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/arch/x86/kernel/sys_x86_64.c#L91
///
/// C signature
///
/// ```
/// void *mmap(
///     unsigned long addr,
///     unsigned long len,
///     unsigned long prot,
///     unsigned long flags,
///     unsigned long fd,
///     unsigned long off,
/// )
/// ```
pub unsafe fn mmap(
    addr: c_ulong,
    len: c_ulong,
    prot: c_ulong,
    flags: c_ulong,
    fd: c_ulong,
    off: c_ulong,
) -> Result<*mut c_void, Error> {
    check!(syscall!(MMAP, addr, len, prot, flags, fd, off)).map(|ret| ret as *mut c_void)
}

// NR = 13
/// Examine and change a signal action
///
/// See `man 2 rt_sigaction` and `man 7 signal` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/signal.c#L3977
///
/// C signature: `int rt_sigaction(int sig, const sigaction *act, sigaction *oact, size_t sigsetsize)`
pub unsafe fn rt_sigaction(
    sig: c_int,
    act: *const sigaction,
    oact: *mut sigaction,
) -> Result<(), Error> {
    check!(syscall!(
        RT_SIGACTION,
        sig,
        act,
        oact,
        mem::size_of::<sigset_t>()
    ))
    .map(|ret| {
        debug_assert_eq!(ret, 0);
    })
}

// NR = 14
/// Examine and change blocked signals
///
/// See `man 2 rt_sigprocmask` and `man 7 signal` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/signal.c#L2881
///
/// C signature:
///
/// ```
/// int rt_sigprocmask(int how, const sigset_t *nset, sigset_t *oset, size_t sigsetsize)
/// ```
pub unsafe fn rt_sigprocmask(
    how: c_int,
    nset: *const sigset_t,
    oset: *mut sigset_t,
) -> Result<(), Error> {
    check!(syscall!(
        RT_SIGPROCMASK,
        how,
        nset,
        oset,
        mem::size_of::<sigset_t>()
    ))
    .map(|ret| {
        debug_assert_eq!(ret, 0);
    })
}

// NR = 24
/// Yield the processor
///
/// See `man 2 sched_yield` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/core.c#L4944
///
/// C signature: `int sched_yield(void)`
pub fn sched_yield() {
    unsafe {
        syscall!(SCHED_YIELD);
    }
}

// NR = 34
/// Wait for a signal
///
/// See `man 2 pause` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/signal.c#4170
///
/// C signature: `int getpid()`
pub fn pause() {
    let _ret = unsafe { syscall!(PAUSE) };
    debug_assert!((_ret as isize) < 0);
}

// NR = 39
/// Get process identification
///
/// See `man 2 getpid` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sys.c#891
///
/// C signature: `pid_t getpid()`
pub fn getpid() -> pid_t {
    unsafe { syscall!(GETPID) as pid_t }
}

// NR = 56
/// Create a child process
///
/// See `man 2 clone` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/fork.c#2310
///
/// C signature:
///
/// ```
/// long clone(
///     unsigned long clone_flags,
///     unsigned long newsp,
///     int *parent_tidptr,
///     int *child_tidptr,
///     unsigned long tls,
/// )
/// ```
#[cfg(unused)] // this seems impossible to use correctly with the Rust ABI
pub unsafe fn clone(
    clone_flags: c_ulong,
    newsp: *mut c_void,
    parent_tid: *mut c_int,
    child_tid: *mut c_int,
    tls: c_ulong,
) -> Result<pid_t, Error> {
    check!(syscall!(
        CLONE,
        clone_flags,
        newsp,
        parent_tid,
        child_tid,
        tls
    ))
    .map(|ret| ret as pid_t)
}

// NR = 56
/// Create a child process -- interface simplified for the x86_64 architecture
///
/// See `man 2 clone` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/fork.c#2310
///
/// C signature:
///
/// ```
/// long clone(
///     unsigned long clone_flags,
///     unsigned long newsp,
///     int *parent_tidptr,
///     int *child_tidptr,
///     unsigned long tls,
/// )
/// ```
#[cfg(target_arch = "x86_64")]
pub unsafe fn x86_64_clone(
    clone_flags: c_ulong,
    newsp: *mut c_void,
    f: extern "C" fn() -> !,
) -> Result<pid_t, Error> {
    extern "C" {
        fn __clone(clone_flags: c_ulong, newsp: *mut c_void, f: extern "C" fn() -> !) -> isize;
    }

    check!(__clone(clone_flags, newsp, f)).map(|ret| ret as pid_t)
}

// This assembly correspond to the following pseudo-code
//
// 0. `fn(clone_flags: c_ulong, mut sp: *mut usize, f: usize)`
// 1. `*sp.offset(-1) = f`
// 2. `sp = sp.offset(-1)`
// 3-4. `sys_clone(clone_flags, sp, /* don't core */)`
// 5. return
//
// In (1) the parent writes `f` (8 bytes) at the top of the child stack -- remember than in x86_64
// the stack grows downwards, towards smaller addresses, and note that `sp` is *not* a valid byte
// address so we have to subtract 8 bytes.
//
// (2-4) does the clone system call. If the call succeeds the child starts with the almost same
// context as the parent except for two things: register `%rax` becomes zero and register `%rsp`
// (the stack pointer) becomes `sp`. The `ret` instruction will pop an (8-byte) address from `%rsp`
// and use that as the new instruction pointer (`%rip`). Because we previously wrote `f` into the
// child stack the `ret` instruction makes the child "return" to `f`.
//
// As for the parent, the `syscall` instruction will write the result of the syscall into the
// register `%rax`, which is also the return value of the `__clone` function
#[cfg(target_arch = "x86_64")]
global_asm!(
    r#"
  .global __clone
  .section .text.__clone
__clone:
  mov    %rdx,-0x8(%rsi)
  add    $0xfffffffffffffff8,%rsi
  mov    $0x38,%eax
  syscall
  retq
"#
);

// NR = 60
/// Terminate the calling process
///
/// See `man 2 exit` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/exit.c#946
///
/// C signature: `void exit(int error_code)`
pub unsafe fn exit(error_code: u8) -> ! {
    syscall!(EXIT, error_code as c_int);

    hint::unreachable_unchecked()
}

// NR = 62
/// Send signal to a process
///
/// See `man 2 kill` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/signal.c#3570
///
/// C signature: `int kill(pid_t pid, int sig)`
pub unsafe fn kill(pid: pid_t, sig: c_int) -> Result<(), Error> {
    check!(syscall!(KILL, pid, sig)).map(|res| {
        debug_assert_eq!(res, 0);
    })
}

// NR = 127
/// Examine pending signals
///
/// See `man 2 rt_sigpending` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/signal.c#2953
///
/// C signature: `int rt_sigpending(sigset_t *uset, size_t sigsetsize)`
pub unsafe fn rt_sigpending() -> Result<sigset_t, Error> {
    let mut set = MaybeUninit::uninit();
    check!(syscall!(
        RT_SIGPENDING,
        set.as_mut_ptr(),
        mem::size_of::<sigset_t>()
    ))
    .map(|ret| {
        debug_assert_eq!(ret, 0);

        set.assume_init()
    })
}

// NR = 128
/// Synchronously wait for queued signals
///
/// See `man 2 rt_sigtimedwait` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/signal.c#3361
///
/// C signature:
///
/// ```
/// int rt_sigtimedwait(
///     const sigset_t *uthese,
///     siginfo_t *info,
///     const timespec* uts,
///     size_t sigsetsize,
/// )
/// ```
/// `void exit(int error_code)`
pub unsafe fn rt_sigtimedwait(
    uthese: &sigset_t,
    uinfo: *mut siginfo_t,
    uts: *const timespec,
) -> Result<c_int, Error> {
    check!(syscall!(
        RT_SIGTIMEDWAIT,
        uthese as *const _,
        uinfo,
        uts,
        mem::size_of::<sigset_t>()
    ))
    .map(|ret| ret as c_int)
}

// NR = 129
/// Queue a signal and data
///
/// See `man 2 rt_sigqueueinfo` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/signal.c#3607
///
/// C signature:
///
/// ```
/// int rt_sigqueueinfo(pid_t tgid, int sig, const siginfo_t *uinfo)
/// ```
pub unsafe fn rt_sigqueueinfo(tgid: pid_t, sig: c_int, uinfo: &siginfo_t) -> Result<(), Error> {
    check!(syscall!(RT_SIGQUEUEINFO, tgid, sig, uinfo as *const _))
        .map(|ret| debug_assert_eq!(ret, 0))
}

// NR = 140
/// Get program scheduling priority
///
/// See `man 2 getpriority` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sys.c#266
///
/// C signature: `int getpriority(int which, int who)`
#[cfg(unused)] // unlikely to ever be used?
pub fn getpriority(which: c_int, who: c_int) -> Result<c_int, Error> {
    unsafe { check!(syscall!(GETPRIORITY, which, who)).map(|ret| ret as c_int) }
}

// NR = 141
/// Set program scheduling priority
///
/// See `man 2 setpriority` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sys.c#196
///
/// C signature: `int setpriority(int which, int who, int niceval)`
#[cfg(unused)] // unlikely to ever be used?
pub unsafe fn setpriority(which: c_int, who: c_int, niceval: c_int) -> Result<(), Error> {
    check!(syscall!(SETPRIORITY, which, who, niceval)).map(|ret| debug_assert_ne!(ret, 0))
}

// NR = 142
/// Set scheduling parameters
///
/// See `man 2 sched_setparam` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sched/core.c#4551
///
/// C signature: `int sched_setparam(pid_t pid, const sched_param *param)`
pub unsafe fn sched_setparam(pid: pid_t, param: &sched_param) -> Result<(), Error> {
    check!(syscall!(SCHED_SETPARAM, pid, param as *const _)).map(|ret| debug_assert_eq!(ret, 0))
}

// NR = 143
/// Get scheduling parameters
///
/// See `man 2 sched_getparam` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sched/core.c#4625
///
/// C signature: `int sched_getparam(pid_t pid, sched_param *param)`
pub fn sched_getparam(pid: pid_t) -> Result<sched_param, Error> {
    unsafe {
        let mut param = MaybeUninit::uninit();
        check!(syscall!(SCHED_GETPARAM, pid, param.as_mut_ptr())).map(move |ret| {
            debug_assert_eq!(ret, 0);

            param.assume_init()
        })
    }
}

// NR = 144
/// Set scheduling policy / parameters
///
/// See `man 2 sched_setscheduler` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sched/core.c#4536
///
/// C signature: `int sched_setscheduler(pid_t pid, sched_param *param)`
pub unsafe fn sched_setscheduler(
    pid: pid_t,
    policy: c_int,
    param: &sched_param,
) -> Result<(), Error> {
    check!(syscall!(SCHED_SETSCHEDULER, pid, policy, param as *const _))
        .map(|ret| debug_assert_eq!(ret, 0))
}

// NR = 145
/// Get scheduling policy / parameters
///
/// See `man 2 sched_getscheduler` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sched/core.c#4596
///
/// C signature: `int sched_getscheduler(pid_t pid)`
#[cfg(unused)] // might be useful for testing
pub unsafe fn sched_getscheduler(pid: pid_t) -> Result<c_int, Error> {
    check!(syscall!(SCHED_GETSCHEDULER, pid)).map(|ret| ret as c_int)
}

// NR = 146
/// Get static priority range
///
/// See `man 2 sched_get_priority_max` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sched/core.c#5163
///
/// C signature: `int sched_get_priority_max(int policy)`
#[cfg(unused)] // will be for sanity checks when we have multiple threads
pub unsafe fn sched_get_priority_max(policy: c_int) -> Result<c_int, Error> {
    check!(syscall!(SCHED_GET_PRIORITY_MAX, policy)).map(|ret| ret as c_int)
}

// NR = 147
/// Get static priority range
///
/// See `man 2 sched_get_priority_min` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sched/core.c#5190
///
/// C signature: `int sched_get_priority_min(int policy)`
#[cfg(unused)] // will be for sanity checks when we have multiple threads
pub unsafe fn sched_get_priority_min(policy: c_int) -> Result<c_int, Error> {
    check!(syscall!(SCHED_GET_PRIORITY_MIN, policy)).map(|ret| ret as c_int)
}

// NR = 186
/// Get thread identification
///
/// See `man 2 gettid` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sys.c#897
///
/// C signature: `pid_t gettid()`
pub fn gettid() -> pid_t {
    unsafe { syscall!(GETTID) as pid_t }
}

// NR = 203
/// Set thread's CPU affinity mask
///
/// See `man 2 sched_setaffinity` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sched/core.c#4854
///
/// C signature:
///
/// ```
/// int sched_setaffinity(pid_t pid, unsigned int len, const unsigned long *user_mask_ptr)
/// ```
pub unsafe fn sched_setaffinity(pid: pid_t, mask: &[c_ulong; 8]) -> Result<(), Error> {
    check!(syscall!(SCHED_SETAFFINITY, pid, mask.len() as c_uint, mask.as_ptr()) as c_int)
        .map(|ret| debug_assert_eq!(ret, 0))
}

// NR = 204
/// Get thread's CPU affinity mask
///
/// See `man 2 sched_getaffinity` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sched/core.c#4906
///
/// C signature:
///
/// ```
/// int sched_getaffinity(pid_t pid, unsigned int len, unsigned long *user_mask_ptr)
/// ```
#[cfg(unused)]
pub unsafe fn sched_getaffinity(pid: pid_t, mask: &mut [c_ulong; 8]) -> Result<&[c_ulong], Error> {
    check!(syscall!(
        SCHED_GETAFFINITY,
        pid,
        mask.len() as c_uint,
        mask.as_mut_ptr()
    ))
    .map(|ret| {
        debug_assert!(ret <= 8);

        slice::from_raw_parts(mask.as_ptr(), ret as usize)
    })
}

// NR = 222
/// Create a POSIX per-process timer
///
/// See `man 2 timer_create` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/time/posix-timers.c#551
///
/// C signature:
///
/// ```
/// int timer_create(
///     clockid_t which_clock,
///     struct sigevent *timer_event_spec,
///     timer_t *created_timer_id,
/// )
/// ```
pub fn timer_create(which_clock: clockid_t, timer_event_spec: &sigevent) -> Result<timer_t, Error> {
    unsafe {
        let mut timer = MaybeUninit::uninit();
        check!(syscall!(
            TIMER_CREATE,
            which_clock,
            timer_event_spec as *const sigevent,
            timer.as_mut_ptr()
        ))
        .map(move |ret| {
            debug_assert_eq!(ret, 0);
            timer.assume_init()
        })
    }
}

// NR = 223
/// Arm / disarm a POSIX per-process timer
///
/// See `man 2 timer_settime` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/time/posix-timers.c#551
///
/// C signature:
///
/// ```
/// int timer_settime(
///     timer_t timer_id,
///     int flags,
///     const itimerspec* new_setting,
///     itimerspec* old_setting,
/// )
/// ```
pub fn timer_settime(
    timer_id: timer_t,
    flags: c_int,
    new_setting: &itimerspec,
    old_setting: *mut itimerspec,
) -> Result<(), Error> {
    unsafe {
        check!(syscall!(
            TIMER_SETTIME,
            timer_id,
            flags,
            new_setting as *const itimerspec,
            old_setting
        ))
        .map(move |ret| {
            debug_assert_eq!(ret, 0);
        })
    }
}

// NR = 228
/// Retrieve the time of the specified clock
///
/// See `man 2 clock_gettime` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/time/posix-timers.c#1032
///
/// C signature: `int clock_gettime(clockid_t which_clock, struct timespec *tp)`
pub fn clock_gettime(which_clock: clockid_t) -> Result<timespec, Error> {
    unsafe {
        let mut tp = MaybeUninit::uninit();
        check!(syscall!(CLOCK_GETTIME, which_clock, tp.as_mut_ptr())).map(move |ret| {
            debug_assert_eq!(ret, 0);
            tp.assume_init()
        })
    }
}

// NR = 229
/// Finds the resolutions of the specified clock
///
/// See `man 2 clock_getres` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/time/posix-timers.c#1073
///
/// C signature: `int clock_gettime(clockid_t which_clock, struct timespec *tp)`
pub fn clock_getres(which_clock: clockid_t) -> Result<timespec, Error> {
    unsafe {
        let mut tp = MaybeUninit::uninit();
        check!(syscall!(CLOCK_GETTIME, which_clock, tp.as_mut_ptr())).map(move |ret| {
            debug_assert_eq!(ret, 0);
            tp.assume_init()
        })
    }
}

// NR = 231
/// Exit all threads in a process
///
/// See `man 2 exit_group` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sched/exit.c#988
///
/// C signature: `int exit_group(int error_code)`
pub fn exit_group(error_code: u8) -> ! {
    unsafe {
        syscall!(EXIT_GROUP, error_code as c_int);

        hint::unreachable_unchecked()
    }
}

// NR = 234
/// Send a signal to a thread
///
/// See `man 2 tgkill` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/signal.c#3748
///
/// C signature: `int tgkill(pid_t tgid, pid_t pid, int sig)`
pub unsafe fn tgkill(tgid: pid_t, pid: pid_t, sig: c_int) -> Result<(), Error> {
    check!(syscall!(TGKILL, tgid, pid, sig)).map(|res| {
        debug_assert_eq!(res, 0);
    })
}

// NR = 297
/// Queue a signal and data
///
/// See `man 2 rt_tgsigqueueinfo` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/signal.c#3647
///
/// C signature:
///
/// ```
/// int rt_tgsigqueueinfo(pid_t tgid, pid_t pid, int sig, const siginfo_t *uinfo)
/// ```
pub unsafe fn rt_tgsigqueueinfo(
    tgid: pid_t,
    tid: pid_t,
    sig: c_int,
    uinfo: &siginfo_t,
) -> Result<(), Error> {
    check!(syscall!(
        RT_TGSIGQUEUEINFO,
        tgid,
        tid,
        sig,
        uinfo as *const _
    ))
    .map(|ret| debug_assert_eq!(ret, 0))
}

// NR = 309
/// Determine CPU and NUMA node on which the calling thread is running
///
/// See `man 2 getcpu` for more details
///
/// Source: https://github.com/torvalds/linux/blob/v5.0/kernel/sys.c#2502
///
/// C signature:
///
/// ```
/// int getcpu(unsigned *cpup, unsigned *nodep, getcpu_cache *unused)
/// ```
pub fn getcpu(cpup: Option<&mut c_uint>, nodep: Option<&mut c_uint>) {
    unsafe {
        let res = syscall!(
            GETCPU,
            cpup.map(|x| x as *mut c_uint).unwrap_or(ptr::null_mut()),
            nodep.map(|x| x as *mut c_uint).unwrap_or(ptr::null_mut())
        );

        debug_assert_eq!(res, 0);
    }
}

/// Thin wrapper around Linux error codes
#[derive(Clone, Copy, PartialEq)]
pub struct Error {
    code: u8,
}

impl Error {
    /// Returns the error code as an integer
    pub fn code(self) -> u8 {
        self.code
    }
}

impl ufmt::uDebug for Error {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        let e = match self.code {
            1 => "EPERM",
            3 => "ESRCH",
            14 => "EFAULT",
            22 => "EINVAL",
            _ => {
                return uwrite!(f, "Error({})", self.code);
            }
        };

        f.write_str(e)
    }
}
