#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![no_main]
#![no_std]

use linux_io::{Stderr, Stdout};
use panic_exit as _;

#[rtfm::app(cores = 2)]
const APP: () = {
    #[idle(core = 0)]
    fn a(_: a::Context) -> ! {
        let mut cpu = u32::max_value();

        for i in 0..64 {
            // userspace delay
            for j in 0..i {
                unsafe {
                    core::ptr::read_volatile(&j);
                }
            }

            linux_sys::getcpu(Some(&mut cpu), None);
            assert_eq!(cpu, 0);

            Stdout.write(b".").ok();

            linux_sys::getcpu(Some(&mut cpu), None);
            assert_eq!(cpu, 0);
        }

        Stdout.write(b"\n").ok();

        // exit only this thread
        unsafe { linux_sys::exit(0) }
    }

    #[idle(core = 1)]
    fn b(_: b::Context) -> ! {
        let mut cpu = u32::max_value();

        for i in 0..64 {
            linux_sys::getcpu(Some(&mut cpu), None);
            assert_eq!(cpu, 1);

            Stderr.write(b"-").ok();

            linux_sys::getcpu(Some(&mut cpu), None);
            assert_eq!(cpu, 1);

            for j in 0..i {
                unsafe {
                    core::ptr::read_volatile(&j);
                }
            }
        }

        Stderr.write(b"\n").ok();

        unsafe { linux_sys::exit(0) }
    }
};
