//! Linux I/O

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![no_std]

use core::{
    slice,
    sync::atomic::{AtomicBool, Ordering},
};

use cty::c_uint;
use linux_sys::Error;

/// *Unbuffered* standard output singleton
pub struct Stdout {
    _private: (),
}

impl Stdout {
    const FILENO: c_uint = 1;

    /// Creates an instance of this singleton
    ///
    /// This constructor only returns `Some` the *first* time it's called
    pub fn take_once() -> Option<Self> {
        static ONCE: AtomicBool = AtomicBool::new(false);

        // NOTE(Ordering) we are dealing with a single core so this out to be OK for now
        if ONCE
            .compare_exchange_weak(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            Some(Stdout { _private: () })
        } else {
            None
        }
    }

    /// Borrows the singleton without performing any synchronization
    pub unsafe fn borrow_unchecked<R>(f: impl FnOnce(&Self) -> R) -> R {
        f(&Stdout { _private: () })
    }

    /// Write a buffer into this writer, returning how many bytes were written.
    pub fn write(&self, buf: &[u8]) -> Result<usize, Error> {
        write(Self::FILENO, buf)
    }

    /// Attempts to write an entire buffer into this writer.
    pub fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        write_all(Self::FILENO, buf)
    }
}

impl ufmt::uWrite for &'_ Stdout {
    type Error = Error;

    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        self.write_all(s.as_bytes())
    }
}

/// *Unbuffered* standard error singleton
pub struct Stderr {
    _private: (),
}

impl Stderr {
    const FILENO: c_uint = 2;

    /// Creates an instance of this singleton
    ///
    /// This constructor only returns `Some` the *first* time it's called
    pub fn take_once() -> Option<Self> {
        static ONCE: AtomicBool = AtomicBool::new(false);

        // NOTE(Ordering) we are dealing with a single core so this out to be OK for now
        if ONCE
            .compare_exchange_weak(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            Some(Stderr { _private: () })
        } else {
            None
        }
    }

    /// Borrows the singleton without performing any synchronization
    pub unsafe fn borrow_unchecked<R>(f: impl FnOnce(&Self) -> R) -> R {
        f(&Stderr { _private: () })
    }

    /// Write a buffer into this writer, returning how many bytes were written.
    pub fn write(&self, buf: &[u8]) -> Result<usize, Error> {
        write(Self::FILENO, buf)
    }

    /// Attempts to write an entire buffer into this writer.
    pub fn write_all(&self, buf: &[u8]) -> Result<(), Error> {
        write_all(Self::FILENO, buf)
    }
}

impl ufmt::uWrite for &'_ Stderr {
    type Error = Error;

    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        self.write_all(s.as_bytes())
    }
}

fn write(fd: c_uint, buf: &[u8]) -> Result<usize, Error> {
    unsafe { Ok(linux_sys::write(fd, buf)?) }
}

fn write_all(fd: c_uint, mut buf: &[u8]) -> Result<(), Error> {
    while !buf.is_empty() {
        let n = write(fd, buf)?;

        buf = unsafe { slice::from_raw_parts(buf.as_ptr().add(n), buf.len() - n) };
    }

    Ok(())
}
