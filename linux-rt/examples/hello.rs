#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::Stdout;
use panic_stderr as _;
use ufmt::uwriteln;

#[linux_rt::entry]
fn main() {
    if let Some(stdout) = Stdout::take_once() {
        uwriteln!(&mut &stdout, "Hello, world!").ok();
    }
}
