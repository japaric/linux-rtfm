#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::Stdout;
use panic_exit as _;
use ufmt::uwriteln;

#[rtfm::app]
const APP: () = {
    static STDOUT: Stdout = ();
    static mut SHARED: u128 = 0;

    #[init(spawn = [foo])]
    fn init(c: init::Context) -> init::LateResources {
        let stdout = Stdout::take_once().unwrap_or_else(|| panic!());

        let rsp = &mut 0; // snapshot of the stack pointer
        uwriteln!(&mut &stdout, "A(%rsp={:?})", rsp as *mut _).ok();

        c.spawn.foo().ok();

        init::LateResources { STDOUT: stdout }
    }

    #[task(priority = 1, resources = [STDOUT, SHARED], spawn = [bar, baz])]
    fn foo(c: foo::Context) {
        let (mut stdout, mut shared, spawn) = (c.resources.STDOUT, c.resources.SHARED, c.spawn);

        uwriteln!(&mut stdout, "B(%rsp={:?})", &mut 0 as *mut _).ok();

        shared.lock(|shared| {
            *shared += 1;

            spawn.bar().ok();

            uwriteln!(&mut stdout, "C(SHARED={})", *shared as u64).ok();

            spawn.baz().ok();
        });

        uwriteln!(&mut stdout, "F").ok();
    }

    #[task(priority = 2, resources = [STDOUT, SHARED])]
    fn bar(mut c: bar::Context) {
        *c.resources.SHARED += 1;

        uwriteln!(
            &mut c.resources.STDOUT,
            "E(%rsp={:?}, SHARED={})",
            &mut 0 as *mut _,
            *c.resources.SHARED as u64,
        )
        .ok();
    }

    #[task(priority = 3, resources = [STDOUT])]
    fn baz(mut c: baz::Context) {
        uwriteln!(&mut c.resources.STDOUT, "D(%rsp={:?})", &mut 0 as *mut _).ok();
    }
};
