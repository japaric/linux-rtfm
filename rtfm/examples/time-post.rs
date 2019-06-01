#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use core::time::Duration;

use linux_io::{process, time::Instant, Stdout};
use panic_exit as _;
use ufmt::uwriteln;
use ufmt_utils::{consts, Ignore, LineBuffered};

#[rtfm::app]
const APP: () = {
    #[init(spawn = [foo])]
    fn init(c: init::Context) {
        c.spawn.foo().unwrap();
    }

    #[task(priority = 2, spawn = [bar])]
    fn foo(c: foo::Context) {
        let before = Instant::now();
        let _ = c.spawn.bar();
        let after = Instant::now();

        print(after.saturating_duration_since(before));
    }

    #[task(priority = 1)]
    fn bar(_: bar::Context) {}
};

#[inline(never)]
fn print(dur: Duration) {
    let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

    // samples 16384
    // quartiles [673.0, 693.0, 750.0]
    // extremes [635.0, 34871.0]
    // std 3748.2
    uwriteln!(&mut stdout, "{}", dur.subsec_nanos()).ok();

    process::exit(0);
}
