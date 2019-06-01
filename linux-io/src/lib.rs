//! Linux I/O

#![deny(missing_docs)]
#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![no_std]

use core::slice;

use cty::c_uint;
use linux_sys::Error;

pub mod process;
pub mod time;

/// *Unbuffered* standard output singleton
pub struct Stdout;

impl Stdout {
    const FILENO: c_uint = 1;

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

impl ufmt::uWrite for Stdout {
    type Error = Error;

    fn write_str(&mut self, s: &str) -> Result<(), Error> {
        self.write_all(s.as_bytes())
    }
}

/// *Unbuffered* standard error singleton
pub struct Stderr;

impl Stderr {
    const FILENO: c_uint = 2;

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

impl ufmt::uWrite for Stderr {
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
