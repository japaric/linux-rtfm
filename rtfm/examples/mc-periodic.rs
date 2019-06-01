#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![no_main]
#![no_std]

use core::time::Duration;

use linux_io::{process, Stderr, Stdout};
use panic_exit as _;

#[rtfm::app(cores = 2)]
const APP: () = {
    #[init(core = 0, spawn = [ping])]
    fn init(c: init::Context) {
        c.spawn.ping().ok();
    }

    #[task(core = 0, schedule = [ping], spawn = [pong])]
    fn ping(c: ping::Context) {
        Stdout.write(b"[0] ping\n").ok();

        c.spawn.pong().ok();
        c.schedule.ping(c.scheduled + Duration::from_secs(1)).ok();
    }

    #[task(core = 1)]
    fn pong(_: pong::Context) {
        static mut COUNT: u8 = 0;

        Stderr.write(b"[1] pong\n").ok();

        *COUNT += 1;

        if *COUNT == 3 {
            process::exit(0);
        }
    }
};
