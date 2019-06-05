#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::Stdout;
use linux_sys::timespec;
use panic_stderr as _;
use ufmt::uwriteln;
use ufmt_utils::{consts, Ignore, LineBuffered};

#[linux_rt::entry]
fn main() {
    unsafe {
        // schedule all threads on the first core
        linux_sys::sched_setaffinity(0, &[1, 0, 0, 0, 0, 0, 0, 0]).unwrap_or_else(|_| panic!());

        // turn ourselves into a "real-time" process
        // linux_sys::sched_setscheduler(
        //     0,
        //     linux_sys::SCHED_FIFO,
        //     &linux_sys::sched_param { sched_priority: 99 },
        // )
        // .unwrap_or_else(|_| panic!());

        let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

        // samples = 1,024
        // quartiles(SCHED_NORMAL) = [1,694; 16,449; 16,636]
        // quartiles(SCHED_FIFO) = [361; 373; 16,534]
        let instants = time();
        let mut min = i64::max_value();
        let mut max = 0;
        for window in instants.windows(2) {
            let before = window[0].unwrap();
            let after = window[1].unwrap();

            let dur = if before.tv_sec == after.tv_sec {
                after.tv_nsec - before.tv_nsec
            } else {
                ((after.tv_sec - before.tv_sec) * 1_000_000_000) + after.tv_nsec - before.tv_nsec
            };

            if dur > max {
                max = dur
            }

            if dur < min {
                min = dur;
            }
        }

        uwriteln!(&mut stdout, "{} {}", min, max).ok();
    }
}

const N: usize = 1_024;

#[inline(never)]
fn time() -> [Option<timespec>; N] {
    let mut xs = [None; N];

    for x in xs.iter_mut() {
        *x = linux_sys::clock_gettime(linux_sys::CLOCK_MONOTONIC).ok();
    }

    xs
}
