#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![no_main]
#![no_std]

use core::{mem, ptr};

use linux_sys::{siginfo_t, timespec, SIGRTMIN};
use panic_stderr as _;

#[linux_rt::entry]
fn main() {
    unsafe {
        // schedule all threads on the first core
        linux_sys::sched_setaffinity(0, &[1, 0, 0, 0, 0, 0, 0, 0]).unwrap_or_else(|_| panic!());

        // block the first real-time signal
        linux_sys::rt_sigprocmask(
            linux_sys::SIG_BLOCK,
            &(1 << (SIGRTMIN - 1)),
            ptr::null_mut(),
        )
        .unwrap_or_else(|_| panic!());

        // raise the first real-time signal
        let tgid = linux_sys::getpid();
        let tid = tgid;
        let mut si: siginfo_t = mem::uninitialized();
        si.si_code = linux_sys::SI_QUEUE;
        si.si_value = 1;
        linux_sys::rt_tgsigqueueinfo(tgid, tid, SIGRTMIN, &si).unwrap_or_else(|_| panic!());

        // check pending signals; the real-time signal should be there
        let ss = 1 << (SIGRTMIN - 1);
        let pending = linux_sys::rt_sigpending().unwrap_or_else(|_| panic!());
        assert_eq!(pending, ss);

        // wait for the signal; this should return immediately
        let mut si: siginfo_t = mem::uninitialized();
        linux_sys::rt_sigtimedwait(
            &ss,
            &mut si,
            &timespec {
                tv_sec: 1,
                tv_nsec: 0,
            },
        )
        .unwrap_or_else(|_| panic!());

        // check the signal payload
        assert_eq!(si.si_value, 1);

        // there should be no pending signals
        assert_eq!(linux_sys::rt_sigpending().unwrap_or_else(|_| panic!()), 0);
    }
}
