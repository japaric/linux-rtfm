#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![no_main]
#![no_std]

use core::time::Duration;

use linux_io::{process, Stdout};
use panic_exit as _;

#[rtfm::app]
const APP: () = {
    #[init(spawn = [foo])]
    fn init(c: init::Context) {
        c.spawn.foo().ok();
    }

    #[task(schedule = [foo])]
    fn foo(c: foo::Context) {
        static mut COUNT: u8 = 0;

        Stdout.write(b".").ok();

        *COUNT += 1;
        if *COUNT >= 3 {
            Stdout.write(b"\n").ok();
            process::exit(0);
        }

        c.schedule.foo(c.scheduled + Duration::from_secs(1)).ok();
    }
};
