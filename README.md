# `linux-rtfm`

> [Experiment] Real Time for The Masses on Linux

This is an implementation of the (single core) [Real Time For the Masses][rtfm]
concurrency model on Linux.

[rtfm]: https://japaric.github.io/cortex-m-rtfm/book/en/

**IMPORTANT** (Currently) this is a `no_std`-only framework. You will *not* be
able to use the standard library.

## Supported API

- Software tasks (`#[task]` API)

- Resources and locking mechanism (`lock` API)

- Message passing (`spawn` API)

Note that `#[idle]` is not supported.

## Example

In this section we'll run [`rtfm/examples/lock.rs`](./rtfm/examples/lock.rs)
which is port of [this example] from the RTFM book.

[this example]: https://japaric.github.io/cortex-m-rtfm/book/en/by-example/resources.html#priorities

``` console
$ # all code requires nightly because of inline / global assembly
$ rustup default nightly

$ T=x86_64-unknown-linux-gnu

$ cd rtfm

$ # you must pass the (redundant) `--target` flag or compilation will fail
$ cargo build --target $T --example lock --release

$ cp ../target/$T/release/examples/lock .

$ # this let the process raise its scheduling priority to a "real-time" level
$ # see `man 3 cap_from_text` for more details
$ sudo setcap cap_sys_nice+ep lock

$ ./lock
```

``` text
A(%rsp=0x7ffdea61d694)
B(%rsp=0x7ffdea61d04c)
C(SHARED=1)
D(%rsp=0x7ffdea61c8e0)
E(%rsp=0x7ffdea61c950, SHARED=2)
F
```

And this is the `strace`. (The `setcap` setting doesn't seem to work through
`strace` so we use `sudo`)

``` console
$ sudo strace ./lock >/dev/null 2>trace

$ cat trace
```

``` text
execve("./lock", ["./lock"], 0x7ffd50c3b480 /* 17 vars */) = 0
getpid()                                = 16640
rt_sigaction(SIGRT_15, {sa_handler=0x201590, sa_mask=[RT_15], sa_flags=SA_RESTORER|SA_SIGINFO, sa_restorer=0x20100f}, NULL, 8) = 0
rt_sigaction(SIGRT_14, {sa_handler=0x2017c0, sa_mask=[RT_14 RT_15], sa_flags=SA_RESTORER|SA_SIGINFO, sa_restorer=0x20100f}, NULL, 8) = 0
rt_sigaction(SIGRT_13, {sa_handler=0x201a30, sa_mask=[RT_13 RT_14 RT_15], sa_flags=SA_RESTORER|SA_SIGINFO, sa_restorer=0x20100f}, NULL, 8) = 0
sched_setaffinity(0, 8, [0])            = 0
sched_setscheduler(0, SCHED_FIFO, [1])  = 0
rt_sigprocmask(SIG_SETMASK, [RTMIN RT_1 RT_2 RT_3 RT_4 RT_5 RT_6 RT_7 RT_8 RT_9 RT_10 RT_11 RT_12 RT_13 RT_14 RT_15], NULL, 8) = 0
write(1, "A(%rsp=", 7)                  = 7
write(1, "0x7fffa816f974", 14)          = 14
write(1, ")\n", 2)                      = 2
rt_sigqueueinfo(16640, SIGRT_15, {})    = 0
rt_sigprocmask(SIG_SETMASK, [], NULL, 8) = 0
--- SIGRT_15 {si_signo=SIGRT_15, si_code=SI_QUEUE, si_errno=1714911280, si_pid=13367, si_uid=0} ---
write(1, "B(%rsp=", 7)                  = 7
write(1, "0x7fffa816f30c", 14)          = 14
write(1, ")\n", 2)                      = 2
rt_sigprocmask(SIG_SETMASK, [RT_14 RT_15], NULL, 8) = 0
rt_sigqueueinfo(16640, SIGRT_14, {})    = 0
write(1, "C(SHARED=", 9)                = 9
write(1, "1", 1)                        = 1
write(1, ")\n", 2)                      = 2
rt_sigqueueinfo(16640, SIGRT_13, {})    = 0
--- SIGRT_13 {si_signo=SIGRT_13, si_code=SI_QUEUE, si_pid=822083584, si_uid=0} ---
write(1, "D(%rsp=", 7)                  = 7
write(1, "0x7fffa816eba0", 14)          = 14
write(1, ")\n", 2)                      = 2
rt_sigreturn({mask=[RT_14 RT_15]})      = 0
rt_sigprocmask(SIG_SETMASK, [RT_15], NULL, 8) = 0
--- SIGRT_14 {si_signo=SIGRT_14, si_code=SI_QUEUE, si_pid=0, si_uid=0} ---
write(1, "E(%rsp=", 7)                  = 7
write(1, "0x7fffa816ec10", 14)          = 14
write(1, ", SHARED=", 9)                = 9
write(1, "2", 1)                        = 1
write(1, ")\n", 2)                      = 2
rt_sigreturn({mask=[RT_15]})            = 0
write(1, "F\n", 2)                      = 2
rt_sigreturn({mask=[]})                 = 0
exit(0)                                 = ?
+++ exited with 0 +++
```

### Smaller binaries

The `*-linux-gnu` targets always produce relocatable code suitable for dynamic
linking (GOT, full relro, etc.) but we are producing statically linked binaries
so we don't need all relocatable stuff. We can produce smaller binaries using
the `x86_64-linux-rtfm` target -- these won't include relocatable bits -- but
they require using [Xargo].

[Xargo]: https://crates.io/crates/xargo

``` console
$ # size of previous binary
$ strip -s lock

$ size lock
   text    data     bss     dec     hex filename
   4694      16      30    4740    1284 lock

$ size -Ax lock
lock  :
section               size       addr
.gcc_except_table     0x28   0x200190
.rodata              0x13a   0x2001b8
.eh_frame            0x28c   0x2002f8
.text                0xe68   0x201000
.data                  0x8   0x202000
.got                   0x8   0x203000
.bss                  0x1e   0x204000
.comment              0x12        0x0
Total               0x1296

$ # size on disk in bytes
$ stat -c %s lock
13032

$ # now we produce the smaller binary
$ export RUST_TARGET_PATH=$(pwd)

$ T=x86_64-linux-rtfm

$ xargo build --target $T --example lock --release

$ cp ../target/$T/release/examples/lock .

$ strip -s lock

$ size lock
   text    data     bss     dec     hex filename
   3944       0      30    3974     f86 lock

$ size -Ax lock
lock  :
section     size       addr
.rodata    0x13a   0x200158
.text      0xe2e   0x201000
.bss        0x1e   0x202000
.comment    0x12        0x0
Total      0xf98

$ stat -c %s lock
8168
```

## Platform support

Only x86_64 is supported at the moment. A few bits of assembly are required to
support other architectures but I currently lack the time to figure out what's
need and test this code on other platforms.

## Implementation

The whole framework is implemented in pure Rust. All required system calls are
done directly using inline assembly; the framework doesn't depend on a C
library so you can link applications using LLD instead of GCC.

On start up the process changes its CPU affinity (see `man 2 sched_setaffinity`)
to core #0, forcing its starting "thread" and all the other threads spawned from
it to run on a single core. The process also changes its scheduling policy (see
`man 2 sched_setscheduler`) to the "real-time" `SCHED_FIFO` policy with the
lowest priority of `1`; this should give the process higher priority over all
other processes running on the system.

Software tasks are implemented on top of "real-time" signal handlers (see `man 7
signal`). Signal masking (see `man 2 rt_sigprocmask`) is used to implement
prioritization of signal handlers and let them preempt each other in a
controlled fashion. Message passing is implemented using the `rt_sigqueueinfo`
(see `man 2 rt_sigqueueinfo`) system call.

Currently the framework spawns no additional threads nor does it let
applications spawn them so all software tasks run on a single core and a single
(call) stack.

## Notes for `self`

It should be possible to implement multi-core RTFM by spawning a second thread
(with its own call stack) and pinning it to the second core (using
`sched_setaffinity`). This second thread (core) would have its own set of signal
handlers (software tasks) and one thread would signal the other thread (for
message passing) using the `rt_tgsigqueueinfo` (see `man 2 rt_tgsigqueueinfo`)
system call. See [`rtfm-rt/examples/thread2.rs`] for a proof of concept minus
the `rt_tgsigqueueinfo` part.

[`rtfm-rt/examples/thread2.rs`]: ./rtfm-rt/examples/thread2.rs

It's possible to configure systemd so that all processes spawned from it have a
specific CPU affinity (see `man 5 systemd.exec`). One could use this to make all
RTFM applications run on a specific core while the rest of non-real-time system
processes run on different cores.

There's a `ioprio_set` system call (see `man 2 ioprio_set`) for changing the I/O
scheduling priority but I haven't experimented with it yet.

I recall reading somewhere that the kernel limits, for "fairness", the amount of
time one process / thread can run under a real-time scheduling policy without
doing any blocking I/O to a certain amount of *microseconds* but this
(system-wide) number is configurable. ~I can no longer find a reference to this
information.~ See section "Limiting the CPU usage of real-time and deadline
processes" in `man 7 sched`

## License

All source code is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  [https://www.apache.org/licenses/LICENSE-2.0][L1])

- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  [https://opensource.org/licenses/MIT][L2])

[L1]: https://www.apache.org/licenses/LICENSE-2.0
[L2]: https://opensource.org/licenses/MIT

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as above, without any additional terms or conditions.
