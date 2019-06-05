#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![no_main]
#![no_std]

use linux_io::Stdout;
use panic_stderr as _;

#[linux_rt::entry]
fn main() {
    Stdout.write(b"Hello, world!\n").ok();
}
