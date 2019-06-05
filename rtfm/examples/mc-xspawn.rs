#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![no_main]
#![no_std]

use linux_io::{process, Stderr, Stdout};
use panic_exit as _;

#[rtfm::app(cores = 2)]
const APP: () = {
    #[init(core = 0, spawn = [ping])]
    fn init(c: init::Context) {
        c.spawn.ping().ok();
    }

    #[task(core = 0, spawn = [ping])]
    fn pong(c: pong::Context) {
        static mut COUNT: u8 = 0;

        *COUNT += 1;

        Stdout.write(b"[0] pong\n").ok();

        if *COUNT == 2 {
            process::exit(0);
        }

        c.spawn.ping().ok();
    }

    #[task(core = 1, spawn = [pong])]
    fn ping(c: ping::Context) {
        Stderr.write(b"[1] ping\n").ok();

        c.spawn.pong().ok();
    }
};
