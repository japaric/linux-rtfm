#![allow(non_camel_case_types)]

use core::cmp::Ordering;

use cty::{c_int, c_long, c_uint, c_ulong, c_void};
use ufmt::derive::uDebug;

/// Clock identifier
pub type clockid_t = c_int;

/// Process identifier
pub type pid_t = c_int;

// Linux 5.0 supports 64 signals
/// Signal set
pub type sigset_t = c_ulong;

/// Timer identifier
pub type timer_t = c_int;

/// Time
#[derive(Clone, Copy, Eq, PartialEq, uDebug)]
#[repr(C)]
pub struct timespec {
    /// Seconds
    pub tv_sec: c_long,

    /// Nanoseconds
    pub tv_nsec: c_long,
}

impl Ord for timespec {
    fn cmp(&self, other: &timespec) -> Ordering {
        let me = (self.tv_sec, self.tv_nsec);
        let other = (other.tv_sec, other.tv_nsec);
        me.cmp(&other)
    }
}

impl PartialOrd for timespec {
    fn partial_cmp(&self, other: &timespec) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Interval timer
#[repr(C)]
pub struct itimerspec {
    /// Timer period
    pub it_interval: timespec,

    /// Timer expiration
    pub it_value: timespec,
}

/* sigaction */
/// Signal handler
pub union sighandler_t {
    /// Simple signal handler
    pub handler: extern "C" fn(c_int),

    /// Signal handler with info
    pub sigaction: extern "C" fn(c_int, &mut siginfo_t, *mut c_void),

    /// Alternative signal handler
    pub sig: Action,
}

/// Alternative signal handling
#[derive(Clone, Copy)]
pub enum Action {
    /// Default action
    DFL = 0,

    /// Ignore signal
    IGN = 1,
}

/// Signal action
#[repr(C)]
pub struct sigaction {
    /// Signal handler
    pub sa_: sighandler_t,

    /// Action flags
    pub sa_flags: c_ulong,

    /// See `man 2 sigreturn`
    pub sa_restorer: Option<unsafe extern "C" fn() -> !>,

    /// Block mask
    pub sa_mask: sigset_t,
}

/// Signal information
#[repr(C)]
#[derive(uDebug)]
pub struct siginfo_t {
    /// Signal number
    pub si_signo: c_int,

    /// errno
    pub si_errno: c_int,

    /// Signal info code (`SI_*`)
    pub si_code: c_int,

    pad0: u32,

    /// Process ID
    pub si_pid: pid_t,

    /// User ID
    pub si_uid: c_uint,

    /// User data
    pub si_value: usize,

    pad1: [u32; 24],
}

const SI_MAX_SIZE: usize = 128;
#[allow(dead_code)]
const ASSERT: [(); 0 - !(core::mem::size_of::<siginfo_t>() == SI_MAX_SIZE) as usize] = [];

/* sigevent */
/// Signal event
#[repr(C)]
pub struct sigevent {
    /// Data passed with notification
    pub sigev_value: sigval_t,

    /// Notification signal
    pub sigev_signo: c_int,

    /// Notification method (one of SIGEV_*)
    pub sigev_notify: c_int,

    /// ID of thread to signal
    pub sigev_tid: c_int,
}

/// `sigev` data
pub union sigval_t {
    /// An integer
    pub sival_int: c_int,

    /// A pointer
    pub sival_ptr: *mut c_void,
}

/* sched_* */
/// Scheduling parameter
#[derive(uDebug)]
#[repr(C)]
pub struct sched_param {
    /// Scheduling priority
    pub sched_priority: c_int,
}
