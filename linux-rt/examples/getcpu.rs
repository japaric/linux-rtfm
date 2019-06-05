#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::Stdout;
use panic_stderr as _;
use ufmt::uwriteln;
use ufmt_utils::{consts, Ignore, LineBuffered};

#[linux_rt::entry]
fn main() {
    let mut stdout = LineBuffered::<_, consts::U100>::new(Ignore::new(Stdout));

    let mut cpu = 0;
    let mut node = 0;
    linux_sys::getcpu(Some(&mut cpu), Some(&mut node));

    uwriteln!(&mut stdout, "cpu={}, node={}", cpu, node).ok();
}
