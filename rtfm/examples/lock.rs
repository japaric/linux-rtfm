#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::{process, Stdout};
use panic_exit as _;
use ufmt::uwriteln;
use ufmt_utils::{consts, Ignore, LineBuffered};

#[rtfm::app]
const APP: () = {
    static mut SHARED: u128 = 0;

    #[init(spawn = [foo])]
    fn init(c: init::Context) {
        let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

        let rsp = &mut 0; // snapshot of the stack pointer
        uwriteln!(&mut stdout, "A(%rsp={:?})", rsp as *mut _).ok();

        c.spawn.foo().ok();
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        process::exit(0);
    }

    #[task(priority = 1, resources = [SHARED], spawn = [bar, baz])]
    fn foo(mut c: foo::Context) {
        let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

        uwriteln!(&mut stdout, "B(%rsp={:?})", &mut 0 as *mut _).ok();

        let spawn = c.spawn;
        c.resources.SHARED.lock(|shared| {
            *shared += 1;

            spawn.bar().ok();

            uwriteln!(&mut stdout, "C(SHARED={})", *shared as u64).ok();

            spawn.baz().ok();
        });

        uwriteln!(&mut stdout, "F").ok();
    }

    #[task(priority = 2, resources = [SHARED])]
    fn bar(c: bar::Context) {
        let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

        *c.resources.SHARED += 1;

        uwriteln!(
            &mut stdout,
            "E(%rsp={:?}, SHARED={})",
            &mut 0 as *mut _,
            *c.resources.SHARED as u64,
        )
        .ok();
    }

    #[task(priority = 3)]
    fn baz(_: baz::Context) {
        let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

        uwriteln!(&mut stdout, "D(%rsp={:?})", &mut 0 as *mut _).ok();
    }
};
