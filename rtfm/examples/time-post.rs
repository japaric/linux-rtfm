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

    #[task(priority = 2, resources = [STDOUT], spawn = [bar])]
    fn foo(c: foo::Context) {
        let before = Instant::now();
        let _ = c.spawn.bar();
        let after = Instant::now();

        print(c.resources.STDOUT, after.saturating_duration_since(before));
    }

    #[task(priority = 1)]
    fn bar(_: bar::Context) {}
};

#[inline(never)]
fn print(mut stdout: &Stdout, dur: Duration) {
    // samples 16384
    // quartiles [673.0, 693.0, 750.0]
    // extremes [635.0, 34871.0]
    // std 3748.2
    uwriteln!(&mut stdout, "{}", dur.subsec_nanos()).ok();
}
