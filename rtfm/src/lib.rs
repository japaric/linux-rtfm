#![deny(warnings)]
#![feature(maybe_uninit)]
#![no_std]

use core::{hint, time::Duration};

pub mod export;

use linux_rt as _;
pub use linux_rtfm_macros::app;
use linux_sys::timespec;

pub trait Mutex {
    type T;

    fn lock<R>(&mut self, f: impl FnOnce(&mut Self::T) -> R) -> R;
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct Instant {
    ts: timespec,
}

impl Instant {
    pub fn now() -> Self {
        Self {
            ts: linux_sys::clock_gettime(linux_sys::CLOCK_MONOTONIC_RAW).unwrap_or_else(
                |_| unsafe {
                    if cfg!(debug_assertions) {
                        panic!()
                    } else {
                        hint::unreachable_unchecked()
                    }
                },
            ),
        }
    }

    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        if self < &earlier {
            None
        } else {
            let (sec, nsec) = if self.ts.tv_nsec >= earlier.ts.tv_nsec {
                (
                    self.ts.tv_sec - earlier.ts.tv_sec,
                    self.ts.tv_nsec - earlier.ts.tv_nsec,
                )
            } else {
                (
                    self.ts.tv_sec - 1 - earlier.ts.tv_sec,
                    self.ts.tv_nsec + 1_000_000_000 - earlier.ts.tv_nsec,
                )
            };

            // NOTE `nsec` is always less than `1_000_000_000`
            // NOTE `sec` is always positive
            Some(Duration::new(sec as u64, nsec as u32))
        }
    }

    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier)
            .unwrap_or(Duration::new(0, 0))
    }
}
