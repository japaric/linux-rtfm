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
        let now = Instant::now();

        let _ = c.spawn.bar(now);
    }

    #[task(priority = 1)]
    fn bar(_: bar::Context, before: Instant) {
        let now = Instant::now();

        print(now.saturating_duration_since(before));
    }
};

#[inline(never)]
fn print(dur: Duration) {
    let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

    // samples 16384
    // quartiles [1708.0, 1752.0, 1965.0]
    // extremes [1609.0, 217872.0]
    // std 4298.72
    uwriteln!(&mut stdout, "{}", dur.subsec_nanos()).ok();
    process::exit(0);
}
