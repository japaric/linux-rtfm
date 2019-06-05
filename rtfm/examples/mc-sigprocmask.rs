#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use core::sync::atomic::{AtomicBool, Ordering};

use linux_io::{process, Stdout};
use panic_exit as _;
use ufmt::uwriteln;
use ufmt_utils::{consts, Ignore, LineBuffered};

static B0: AtomicBool = AtomicBool::new(false);
static B1: AtomicBool = AtomicBool::new(false);

#[rtfm::app(cores = 2)]
const APP: () = {
    static mut X: i32 = 0;

    #[init(core = 0, spawn = [a])]
    fn init(c: init::Context) {
        c.spawn.a().ok();
    }

    #[task(core = 0, resources = [X], spawn = [c])]
    fn a(mut c: a::Context) {
        let spawn = c.spawn;
        // block RT_MIN
        c.resources.X.lock(|_| {
            spawn.c().ok();

            while !B0.load(Ordering::Acquire) {}
        });
        // unblock RT_MIN

        c.spawn.c().ok();

        while !B1.load(Ordering::Acquire) {}

        process::exit(0);
    }

    #[task(core = 0, priority = 2, resources = [X])]
    fn b(_: b::Context) {}

    #[task(core = 1)]
    fn c(_: c::Context) {
        let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

        let mut sigset = 0;
        unsafe {
            linux_sys::rt_sigprocmask(linux_sys::SIG_BLOCK, &0, &mut sigset)
                .unwrap_or_else(|_| panic!());
        }

        // sanity check that this prints the same regardless of what the core does to its own signal
        // mask
        uwriteln!(&mut stdout, "{:?}", sigset as *const u8).ok();

        if B0.swap(true, Ordering::Release) {
            B1.store(true, Ordering::Release);
        }
    }
};
