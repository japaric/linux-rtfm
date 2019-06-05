#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use core::ptr;

use linux_io::{time::Instant, Stdout};
use linux_sys::{itimerspec, sigaction, sigevent, sighandler_t, sigval_t, timespec, SIGRTMIN};
use panic_stderr as _;
use ufmt::uwriteln;
use ufmt_utils::{consts, Ignore, LineBuffered};

#[linux_rt::entry]
fn main() {
    let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

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

        let timer = linux_sys::timer_create(
            linux_sys::CLOCK_MONOTONIC,
            &sigevent {
                sigev_value: sigval_t { sival_int: 0 },
                sigev_signo: SIGRTMIN,
                sigev_notify: linux_sys::SIGEV_SIGNAL,
                sigev_tid: 0,
            },
        )
        .unwrap_or_else(|_| panic!());

        let now: timespec = Instant::now().into();
        linux_sys::timer_settime(
            timer,
            linux_sys::TIMER_ABSTIME,
            &itimerspec {
                it_interval: timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: timespec {
                    tv_sec: now.tv_sec + 2,
                    tv_nsec: 0,
                },
            },
            ptr::null_mut(),
        )
        .unwrap_or_else(|_| panic!());
        uwriteln!(&mut stdout, "before handler @ {:?}", now).ok();

        linux_sys::pause();

        let now = timespec::from(Instant::now());
        uwriteln!(&mut stdout, "after handler @ {:?}", now).ok();
    }
}

extern "C" fn handler(_sig: i32) {
    let now = timespec::from(Instant::now());
    let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

    uwriteln!(&mut stdout, "handler @ {:?}", now).ok();
}
