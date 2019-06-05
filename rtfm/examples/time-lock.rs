#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::{process, time::Instant, Stdout};
use panic_exit as _;
use ufmt::uwriteln;
use ufmt_utils::{consts, Ignore, LineBuffered};

#[rtfm::app]
const APP: () = {
    static mut SHARED: u128 = 0;

    #[init(spawn = [foo])]
    fn init(c: init::Context) {
        c.spawn.foo().unwrap();
    }

    #[task(priority = 1, resources = [SHARED])]
    fn foo(mut c: foo::Context) {
        let before = Instant::now();
        let inside = c.resources.SHARED.lock(|_shared| Instant::now());
        let after = Instant::now();

        print(before, inside, after);
    }

    #[task(priority = 2, resources = [SHARED])]
    fn bar(_: bar::Context) {}
};

#[inline(never)]
fn print(before: Instant, inside: Instant, after: Instant) {
    let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

    // samples 32768
    // quartiles [526.0, 540.0, 563.0]
    // extremes [489.0, 31813.0]
    // std 1328.32
    uwriteln!(
        &mut stdout,
        "{} {}",
        inside.saturating_duration_since(before).subsec_nanos(),
        after.saturating_duration_since(inside).subsec_nanos(),
    )
    .ok();

    process::exit(0);
}
