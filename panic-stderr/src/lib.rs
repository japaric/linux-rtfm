//! Panic handler: print panic location to stderr and exit the program

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_std]

use core::{convert::Infallible, panic::PanicInfo};

use heapless::{consts, String};
use linux_io::Stderr;
use ufmt::{uWrite, uwrite};

struct Buffer(String<consts::U100>);

impl uWrite for Buffer {
    type Error = Infallible;

    fn write_str(&mut self, s: &str) -> Result<(), Infallible> {
        self.0.push_str(s).ok();
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    let mut buffer = Buffer(String::new());
    buffer.0.push_str("panicked").ok();
    if let Some(loc) = info.location() {
        buffer.0.push_str(" at ").ok();
        buffer.0.push_str(loc.file()).ok();
        uwrite!(&mut buffer, ":{}:{}", loc.line(), loc.column()).ok();
    }
    buffer.0.push_str("\n").ok();

    // NOTE *single* `write` system call
    Stderr.write(buffer.0.as_bytes()).ok();

    linux_sys::exit_group(101)
}
