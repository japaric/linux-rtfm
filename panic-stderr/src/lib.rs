//! Panic handler: print panic location to stderr and exit the program

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_std]

use core::panic::PanicInfo;

use linux_io::Stderr;
use ufmt::{uwrite, uwriteln};

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    unsafe {
        Stderr::borrow_unchecked(|mut stderr| {
            if uwrite!(&mut stderr, "panicked at").is_ok() {
                if let Some(loc) = info.location() {
                    uwriteln!(
                        &mut stderr,
                        " {}:{}:{}",
                        loc.file(),
                        loc.line(),
                        loc.column()
                    )
                    .ok();
                } else {
                    uwrite!(&mut stderr, "\n").ok();
                }
            }
        })
    }

    linux_sys::exit_group(101)
}
