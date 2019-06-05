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
    #[init(spawn = [foo])]
    fn init(c: init::Context) {
        Stdout.write(b"init\n").ok();

        c.spawn.foo(42).unwrap();
    }

    #[task]
    fn foo(_: foo::Context, x: u32) {
        let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

        uwriteln!(&mut stdout, "foo({})", x).ok();

        process::exit(0);
    }
};
