#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use core::time::Duration;

use linux_io::Stdout;
use panic_exit as _;
use rtfm::Instant;
use ufmt::uwriteln;

#[rtfm::app]
const APP: () = {
    static STDOUT: Stdout = ();

    #[init(spawn = [foo])]
    fn init(c: init::Context) -> init::LateResources {
        let stdout = Stdout::take_once().unwrap_or_else(|| panic!());

        c.spawn.foo().unwrap_or_else(|_| panic!());

        init::LateResources { STDOUT: stdout }
    }

    #[task(priority = 2, spawn = [bar])]
    fn foo(c: foo::Context) {
        let now = Instant::now();

        let _ = c.spawn.bar(now);
    }

    #[task(priority = 1, resources = [STDOUT])]
    fn bar(c: bar::Context, before: Instant) {
        let now = Instant::now();

        print(c.resources.STDOUT, now.saturating_duration_since(before));
    }
};

#[inline(never)]
fn print(mut stdout: &Stdout, dur: Duration) {
    // samples 16384
    // quartiles [1708.0, 1752.0, 1965.0]
    // extremes [1609.0, 217872.0]
    // std 4298.72
    uwriteln!(&mut stdout, "{}", dur.subsec_nanos()).ok();
}
