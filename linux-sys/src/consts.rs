use cty::{c_int, c_ulong, size_t};

use crate::types::clockid_t;

/* 9. mmap */
// from:
// - include/uapi/asm-generic/mman-common.h
// - include/uapi/asm-generic/mman.h

/// Page can be read
pub const PROT_READ: c_ulong = 0x1;

/// Page can be written
pub const PROT_WRITE: c_ulong = 0x2;

/// Don't use a file
pub const MAP_ANONYMOUS: c_ulong = 0x20;

/// Stack-like segment
pub const MAP_GROWSDOWN: c_ulong = 0x0100;

/// Changes are private
pub const MAP_PRIVATE: c_ulong = 0x02;

/// For anonymous mmap, memory could be uninitialized
pub const MAP_UNINITIALIZED: c_ulong = 0x4000000;

/* 13. rt_sigaction */
// from:
// - arch/x86/include/uapi/asm/signal.h
// - include/uapi/asm-generic/signal-defs.h
/// `sa_sigaction` is used instead of `sa_handler`
#[cfg(target_arch = "x86_64")]
pub const SA_SIGINFO: c_ulong = 0x00000004;

/// Make certain system calls restartable across signals
#[cfg(target_arch = "x86_64")]
pub const SA_RESTART: c_ulong = 0x10000000;

/// See `man 2 sigreturn`
#[cfg(target_arch = "x86_64")]
pub const SA_RESTORER: c_ulong = 0x04000000;

/// Default signal handling
pub const SIG_DFL: size_t = 0;

/// Ignore signal
pub const SIG_IGN: size_t = 1;

/// Smallest real-time signal
pub const SIGRTMIN: c_int = 32;

/* 14. sigprocmask */
/// Additionally block these signals
pub const SIG_BLOCK: c_int = 0;

/// Unblock these signals
pub const SIG_UNBLOCK: c_int = 1;

/// Block only these signals
pub const SIG_SETMASK: c_int = 2;

/* 56. clone */
// from include/uapi/linux/sched.h
/// Set if VM shared between processes
pub const CLONE_VM: c_ulong = 0x00000100;

/// Same thread group?
pub const CLONE_THREAD: c_ulong = 0x00010000;

/// Set if handlers and blocked signals shared
pub const CLONE_SIGHAND: c_ulong = 0x00000800;

/* 129. rt_sigqueueinfo */
/// Signal issued by rt_sigqueueinfo
pub const SI_QUEUE: c_int = -1;

/* 14*. sched_* */
/// Standard round-robin time-sharing policy
pub const SCHED_NORMAL: c_int = 0;

/// First-in, first-out policy
pub const SCHED_FIFO: c_int = 1;

/// Round robin policy
pub const SCHED_RR: c_int = 2;

/// Batch style execution of processes
pub const SCHED_BATCH: c_int = 3;

/// Very low priority background jobs
pub const SCHED_IDLE: c_int = 5;

/* 22*. clock_* */
// from include/uapi/linux/time.h
/// Monotonic timer
pub const CLOCK_MONOTONIC: clockid_t = 1;

/// Similar to CLOCK_MONOTIC but provides access to a raw hardware-based time
pub const CLOCK_MONOTONIC_RAW: clockid_t = 4;

/// A faster but less precise version of CLOCK_MONOTONIC
pub const CLOCK_MONOTONIC_COARSE: clockid_t = 6;
