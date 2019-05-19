#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use core::{mem, ptr};

use linux_io::{Stderr, Stdout};
use linux_sys::{sigaction, sighandler_t, siginfo_t, SIGRTMIN};
use panic_stderr as _;
use ufmt::uwriteln;

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
                sa_: sighandler_t { handler },
                sa_flags: linux_sys::SA_RESTORER,
                sa_restorer: Some(__restorer),
                sa_mask: 0,
            },
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

        // `handler` should run at this point

        if let Some(mut stderr) = Stderr::take_once().as_ref() {
            uwriteln!(&mut stderr, "returned from handler").ok();
        }
    }
}

extern "C" fn handler(sig: i32) {
    if let Some(mut stdout) = Stdout::take_once().as_ref() {
        uwriteln!(&mut stdout, "handler(sig={})", sig).ok();
    }
}
