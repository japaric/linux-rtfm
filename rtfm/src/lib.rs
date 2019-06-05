#![deny(warnings)]
#![no_std]

pub mod export;
mod tq;

pub use linux_io::time::Instant;
use linux_rt as _;
pub use linux_rtfm_macros::app;
pub use rtfm_core::Mutex;
