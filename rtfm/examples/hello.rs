#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::Stdout;
use panic_exit as _;
use ufmt::uwriteln;

#[rtfm::app]
const APP: () = {
    #[init]
    fn init(_: init::Context) {
        if let Some(mut stdout) = Stdout::take_once().as_ref() {
            uwriteln!(&mut stdout, "Hello, world!").ok();
        }
    }
};
