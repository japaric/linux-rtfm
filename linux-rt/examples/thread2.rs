//! Runs two threads; each one on its own CPU

#![deny(rust_2018_compatibility)]
#![deny(rust_2018_idioms)]
#![deny(warnings)]
#![feature(proc_macro_hygiene)]
#![no_main]
#![no_std]

use linux_io::{Stderr, Stdout};
use panic_stderr as _;
use ufmt::uwriteln;

// `$ getconf PAGE_SIZE`
const PAGE_SIZE: u64 = 4_096;
const STACK_SIZE: u64 = 4 * PAGE_SIZE;

#[linux_rt::entry]
fn main() {
    unsafe {
        let mut stdout = &Stdout::take_once().unwrap_or_else(|| panic!());

        // schedule this thread on the first core
        linux_sys::sched_setaffinity(0, &[1 << 0, 0, 0, 0, 0, 0, 0, 0])
            .unwrap_or_else(|_| panic!());

        // turn ourselves into a "real-time" process
        linux_sys::sched_setscheduler(
            0,
            linux_sys::SCHED_FIFO,
            &linux_sys::sched_param { sched_priority: 1 },
        )
        .unwrap_or_else(|_| {
            fatal(
                "error: couldn't change scheduling policy; \
                 run `sudo setcap cap_sys_nice+ep $binary` first\n",
            )
        });

        let stack_low = linux_sys::mmap(
            0,          // address; 0 means any page-aligned address
            STACK_SIZE, // length of mapping
            linux_sys::PROT_READ | // read access
            linux_sys::PROT_WRITE, // write access
            linux_sys::MAP_ANONYMOUS | // mapping is not backed by any file
            linux_sys::MAP_PRIVATE | // mapping is private to other threads / processes
            linux_sys::MAP_GROWSDOWN | // mapping suitable for stacks
            linux_sys::MAP_UNINITIALIZED, // leave memory uninitialized
            !0,         // file descriptor; needs to be `-1` because of MAP_ANONYMOUS
            0,          // offset; ignored because of MAP_ANONYMOUS
        )
        .unwrap_or_else(|_| panic!());

        // the stack grows downwards so we must pass the highest address of the mapping to `clone`
        let stack_high = (stack_low as u64 + STACK_SIZE) as *mut _;

        // spin a new thread
        let child_tid = linux_sys::x86_64_clone(
            linux_sys::CLONE_VM | // new thread shares memory with the parent
            linux_sys::CLONE_THREAD | // share thread group
            linux_sys::CLONE_SIGHAND, // shared signal handlers; required by `CLONE_THREAD`
            stack_high,
            child,
        )
        .unwrap_or_else(|e| {
            uwriteln!(&mut stdout, "{:?}", e).ok();
            panic!()
        });

        // the child process ("thread") will inherit the scheduling policy and priority and the
        // CPU affinity so it will not run just yet

        // make the child thread run on core #1
        linux_sys::sched_setaffinity(child_tid, &[1 << 1, 0, 0, 0, 0, 0, 0, 0])
            .unwrap_or_else(|_| panic!());

        // now `child` will run *in parallel* to this thread / core

        // this output will interleave with the output of the `child`
        for _ in 0..128 {
            stdout.write(&[b'0']).ok();
        }

        stdout.write(&[b'\n']).ok();
    }
}

// code that the child thread will run
extern "C" fn child() -> ! {
    unsafe {
        Stdout::borrow_unchecked(|stdout| {
            for _ in 0..128 {
                stdout.write(&[b'1']).ok();
            }

            stdout.write(&[b'\n']).ok();
        });

        // exit this thread
        linux_sys::exit(0);
    }
}

fn fatal(s: &str) -> ! {
    unsafe {
        Stderr::borrow_unchecked(|stderr| {
            stderr.write_all(s.as_bytes()).ok();
        });

        linux_sys::exit_group(101)
    }
}
