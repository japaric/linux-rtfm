//! Check that real-time signals issued by `kill` are also queued

#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use core::ptr;

use linux_io::Stderr;
use linux_sys::{cty::c_void, sigaction, sighandler_t, siginfo_t, SIGRTMIN};
use panic_stderr as _;

#[linux_rt::entry]
fn main() {
    unsafe {
        // schedule all threads on the first core
        linux_sys::sched_setaffinity(0, &[1, 0, 0, 0, 0, 0, 0, 0]).unwrap_or_else(|_| panic!());

        extern "C" {
            fn __restorer() -> !;
        }

        // set a signal handler for the first real time signal
        linux_sys::rt_sigaction(
            SIGRTMIN,
            &sigaction {
                sa_: sighandler_t { sigaction },
                sa_flags: linux_sys::SA_RESTORER | linux_sys::SA_SIGINFO,
                sa_restorer: Some(__restorer),
                sa_mask: 0,
            },
            ptr::null_mut(),
        )
        .unwrap_or_else(|_| panic!());

        // block the first real-time signal
        linux_sys::rt_sigprocmask(
            linux_sys::SIG_BLOCK,
            &(1 << (SIGRTMIN - 1)),
            ptr::null_mut(),
        )
        .unwrap_or_else(|_| panic!());

        // raise the first real-time signal *twice*
        linux_sys::kill(0, SIGRTMIN).unwrap_or_else(|_| panic!());
        linux_sys::kill(0, SIGRTMIN).unwrap_or_else(|_| panic!());

        // unblock the first real-time signal
        linux_sys::rt_sigprocmask(
            linux_sys::SIG_UNBLOCK,
            &(1 << (SIGRTMIN - 1)),
            ptr::null_mut(),
        )
        .unwrap_or_else(|_| panic!());
    }
}

extern "C" fn sigaction(_sig: i32, _si: &mut siginfo_t, _: *mut c_void) {
    Stderr.write(b"sigaction\n").ok();
}
