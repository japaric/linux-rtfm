#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::Stdout;
use panic_exit as _;
use rtfm::Instant;
use ufmt::uwriteln;

#[rtfm::app]
const APP: () = {
    static STDOUT: Stdout = ();
    static mut SHARED: u128 = 0;

    #[init(spawn = [foo])]
    fn init(c: init::Context) -> init::LateResources {
        let stdout = Stdout::take_once().unwrap_or_else(|| panic!());

        c.spawn.foo().unwrap_or_else(|_| panic!());

        init::LateResources { STDOUT: stdout }
    }

    #[task(priority = 1, resources = [STDOUT, SHARED])]
    fn foo(mut c: foo::Context) {
        let before = Instant::now();
        let inside = c.resources.SHARED.lock(|_shared| Instant::now());
        let after = Instant::now();

        print(c.resources.STDOUT, before, inside, after);
    }

    #[task(priority = 2, resources = [SHARED])]
    fn bar(_: bar::Context) {}
};

#[inline(never)]
fn print(mut stdout: &Stdout, before: Instant, inside: Instant, after: Instant) {
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
}
