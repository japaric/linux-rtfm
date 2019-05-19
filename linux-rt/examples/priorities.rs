#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use core::{mem, ptr};

use cty::c_void;
use linux_io::Stdout;
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

        // set signal handlers for the first thread real time signals
        linux_sys::rt_sigaction(
            SIGRTMIN,
            &sigaction {
                sa_: sighandler_t { sigaction: p1 },
                sa_flags: linux_sys::SA_RESTORER | linux_sys::SA_SIGINFO,
                sa_restorer: Some(__restorer),
                // highest priority, blocks the other two RT signals
                sa_mask: (1 << SIGRTMIN) | (1 << SIGRTMIN + 1),
            },
            ptr::null_mut(),
        )
        .unwrap_or_else(|_| panic!());

        linux_sys::rt_sigaction(
            SIGRTMIN + 1,
            &sigaction {
                sa_: sighandler_t { sigaction: p2 },
                sa_flags: linux_sys::SA_RESTORER | linux_sys::SA_SIGINFO,
                sa_restorer: Some(__restorer),
                // mid priority, blocks the lowest priority RT signal
                sa_mask: (1 << SIGRTMIN + 1),
            },
            ptr::null_mut(),
        )
        .unwrap_or_else(|_| panic!());

        linux_sys::rt_sigaction(
            SIGRTMIN + 2,
            &sigaction {
                sa_: sighandler_t { sigaction: p3 },
                sa_flags: linux_sys::SA_RESTORER | linux_sys::SA_SIGINFO,
                sa_restorer: Some(__restorer),
                // lowest priority, doesn't block any other RT signal
                sa_mask: 0,
            },
            ptr::null_mut(),
        )
        .unwrap_or_else(|_| panic!());

        // raise the second real-time signal
        let tgid = linux_sys::getpid();
        let tid = tgid;
        let mut si: siginfo_t = mem::uninitialized();
        si.si_code = linux_sys::SI_QUEUE;
        si.si_value = 1;
        linux_sys::rt_tgsigqueueinfo(tgid, tid, SIGRTMIN + 1, &si).unwrap_or_else(|_| panic!());

        // `p2` should run at this point

        Stdout::borrow_unchecked(|mut stdout| {
            uwriteln!(&mut stdout, "returned from signal handlers").ok();
        });
    }
}

extern "C" fn p1(sig: i32, si: &mut siginfo_t, _: *mut c_void) {
    unsafe {
        Stdout::borrow_unchecked(|mut stdout| {
            let sp = &mut 0;
            uwriteln!(
                &mut stdout,
                "p1(sig={}, si={}, sp={:?})",
                sig,
                si.si_value,
                sp as *mut i32,
            )
            .ok();
        });
    }
}

extern "C" fn p2(sig: i32, si: &mut siginfo_t, _: *mut c_void) {
    unsafe {
        Stdout::borrow_unchecked(|mut stdout| {
            let sp = &mut 0;
            uwriteln!(
                &mut stdout,
                "p2(sig={}, si={}, sp={:?})",
                sig,
                si.si_value,
                sp as *mut i32,
            )
            .ok();

            // raise the third (lowest priority) RT signal
            let tgid = linux_sys::getpid();
            let tid = tgid;
            si.si_value += 1;
            linux_sys::rt_tgsigqueueinfo(tgid, tid, SIGRTMIN + 2, &si).unwrap_or_else(|_| panic!());

            uwriteln!(&mut stdout, "after raise(SIGRTMIN+2)").ok();

            // raise the first (highest priority) RT signal
            si.si_value += 1;
            linux_sys::rt_tgsigqueueinfo(tgid, tid, SIGRTMIN, &si).unwrap_or_else(|_| panic!());

            uwriteln!(&mut stdout, "after raise(SIGRTMIN)").ok();
        });
    }
}

extern "C" fn p3(sig: i32, si: &mut siginfo_t, _: *mut c_void) {
    unsafe {
        Stdout::borrow_unchecked(|mut stdout| {
            let sp = &mut 0;
            uwriteln!(
                &mut stdout,
                "p3(sig={}, si={}, sp={:?})",
                sig,
                si.si_value,
                sp as *mut i32,
            )
            .ok();
        });
    }
}
