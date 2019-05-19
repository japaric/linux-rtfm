#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::Stdout;
use panic_exit as _;
use ufmt::uwriteln;

#[rtfm::app]
const APP: () = {
    static STDOUT: Stdout = ();

    #[init(spawn = [foo])]
    fn init(c: init::Context) -> init::LateResources {
        let stdout = Stdout::take_once().unwrap_or_else(|| panic!());

        uwriteln!(&mut &stdout, "init").unwrap_or_else(|_| panic!());

        c.spawn.foo(42).unwrap_or_else(|_| panic!());

        init::LateResources { STDOUT: stdout }
    }

    #[task(resources = [STDOUT])]
    fn foo(mut c: foo::Context, x: u32) {
        uwriteln!(&mut c.resources.STDOUT, "foo({})", x).unwrap_or_else(|_| panic!());
    }
};
