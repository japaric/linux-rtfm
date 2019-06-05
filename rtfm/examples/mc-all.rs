//! Use all cores to do nothing useful

#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![no_main]
#![no_std]

use panic_exit as _;

// I have 8 cores; you may need to use a different number
#[rtfm::app(cores = 8)]
const APP: () = {
    #[idle(core = 0)]
    fn a(_: a::Context) -> ! {
        unsafe {
            // exit *this* thread
            linux_sys::exit(0);
        }
    }

    #[idle(core = 1)]
    fn b(_: b::Context) -> ! {
        unsafe { linux_sys::exit(0) }
    }

    #[idle(core = 2)]
    fn c(_: c::Context) -> ! {
        unsafe { linux_sys::exit(0) }
    }

    #[idle(core = 3)]
    fn d(_: d::Context) -> ! {
        unsafe { linux_sys::exit(0) }
    }

    #[idle(core = 4)]
    fn e(_: e::Context) -> ! {
        unsafe { linux_sys::exit(0) }
    }

    #[idle(core = 5)]
    fn f(_: f::Context) -> ! {
        unsafe { linux_sys::exit(0) }
    }

    #[idle(core = 6)]
    fn g(_: g::Context) -> ! {
        unsafe { linux_sys::exit(0) }
    }

    #[idle(core = 7)]
    fn h(_: h::Context) -> ! {
        unsafe { linux_sys::exit(0) }
    }
};
