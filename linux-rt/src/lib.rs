//! Minimal Linux runtime

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(global_asm)]
#![no_std]

mod symbols;

pub use linux_rt_macros::entry;

#[cfg(not(target_arch = "x86_64"))]
compile_error!("Only x86_64 is currently supported");

#[allow(unused_attributes)]
#[no_mangle]
unsafe extern "C" fn start(_stack_top: *const u32) -> ! {
    extern "Rust" {
        fn main();
    }

    main();

    // exit only *this* thread; the user may spawn more in `main`
    linux_sys::exit(0)
}
