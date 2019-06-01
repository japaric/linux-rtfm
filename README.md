# `linux-rtfm`

> [Experiment] Real Time for The Masses on Linux

This is a Linux implementation of the [Real Time For the Masses][rtfm]
concurrency model.

[rtfm]: https://japaric.github.io/cortex-m-rtfm/book/en/

**IMPORTANT** (Currently) this is a `no_std`-only framework. You will *not* be
able to use the standard library.

## Supported API

- Software tasks (`#[task]` API)

- Resources and locking mechanism (`lock` API)

- Message passing (`spawn` API)

- Timer queue (`schedule` API)

- Multi-core support (`cores` API)

## Examples

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
A(%rsp=0x7ffed546746c)
B(%rsp=0x7ffed5466e1c)
C(SHARED=1)
D(%rsp=0x7ffed5466614)
E(%rsp=0x7ffed546667c, SHARED=2)
F
```

And this is the `strace`. The `setcap` setting doesn't seem to work through
`strace` so we have to use `sudo`

``` console
$ sudo strace ./lock >/dev/null
```

``` text
execve("./lock", ["./lock"], 0x7ffc2ac460c0 /* 17 vars */) = 0
sched_setaffinity(0, 8, [0])            = 0
sched_setscheduler(0, SCHED_FIFO, [1])  = 0
rt_sigprocmask(SIG_BLOCK, [RTMIN RT_1 RT_2], NULL, 8) = 0
getpid()                                = 5850
rt_sigaction(SIGRT_2, {sa_handler=0x201890, sa_mask=[], sa_flags=SA_RESTORER|SA_SIGINFO, sa_restorer=0x20100f}, NULL, 8) = 0
rt_sigaction(SIGRT_1, {sa_handler=0x201a80, sa_mask=[RT_2], sa_flags=SA_RESTORER|SA_SIGINFO, sa_restorer=0x20100f}, NULL, 8) = 0
rt_sigaction(SIGRTMIN, {sa_handler=0x201c90, sa_mask=[RT_1 RT_2], sa_flags=SA_RESTORER|SA_SIGINFO, sa_restorer=0x20100f}, NULL, 8) = 0
write(1, "A(%rsp=0x7ffedee66b7c)\n", 23) = 23
rt_sigqueueinfo(5850, SIGRT_2, {})      = 0
rt_sigprocmask(SIG_UNBLOCK, [RTMIN RT_1 RT_2], NULL, 8) = 0
--- SIGRT_2 {si_signo=SIGRT_2, si_code=SI_QUEUE, si_errno=1714911280, si_pid=25399, si_uid=0} ---
write(1, "B(%rsp=0x7ffedee6651c)\n", 23) = 23
rt_sigprocmask(SIG_BLOCK, [RT_1], NULL, 8) = 0
rt_sigqueueinfo(5850, SIGRT_1, {})      = 0
write(1, "C(SHARED=1)\n", 12)           = 12
rt_sigqueueinfo(5850, SIGRTMIN, {})     = 0
--- SIGRTMIN {si_signo=SIGRTMIN, si_code=SI_QUEUE, si_pid=822083584, si_uid=0} ---
write(1, "D(%rsp=0x7ffedee65d14)\n", 23) = 23
rt_sigreturn({mask=[RT_1 RT_2]})        = 0
rt_sigprocmask(SIG_UNBLOCK, [RT_1], NULL, 8) = 0
--- SIGRT_1 {si_signo=SIGRT_1, si_code=SI_QUEUE, si_pid=0, si_uid=0} ---
write(1, "E(%rsp=0x7ffedee65d7c, SHARED=2)"..., 33) = 33
rt_sigreturn({mask=[RT_2]})             = 0
write(1, "F\n", 2)                      = 2
rt_sigreturn({mask=[]})                 = 0
exit_group(0)                           = ?
+++ exited with 0 +++
```

### Multi-core

There are also multi-core examples in the `rtfm` directory; all of them are
named with an `mc-` prefix and most of them assume that the target system has
at least 2 cores. Running them is no different that running a single core
example; however, if you are going to `strace` these binaries don't forget to
use the `-f` flag or you won't see all the system calls.

`mc-xspawn` is the classic ping pong message passing application.

``` console
$ ./mc-xspawn
[1] ping
[0] pong
[1] ping
[0] pong
```

The number inside the square brackets is the core number.

And this is the `strace`

``` console
$ sudo strace -f ./mc-xspawn
```

``` text
execve("./mc-xspawn", ["./mc-xspawn"], 0x7fff64483578 /* 17 vars */) = 0
sched_setaffinity(0, 8, [0])            = 0
sched_setscheduler(0, SCHED_FIFO, [1])  = 0
rt_sigprocmask(SIG_BLOCK, [RTMIN RT_1], NULL, 8) = 0
getpid()                                = 4617
rt_sigaction(SIGRTMIN, {sa_handler=0x201090, sa_mask=[], sa_flags=SA_RESTORER|SA_SIGINFO, sa_restorer=0x20100f}, NULL, 8) = 0
rt_sigaction(SIGRT_1, {sa_handler=0x201150, sa_mask=[], sa_flags=SA_RESTORER|SA_SIGINFO, sa_restorer=0x20100f}, NULL, 8) = 0
mmap(NULL, 8388608, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS|MAP_GROWSDOWN|1<<MAP_HUGE_SHIFT, -1, 0) = 0x7f48cbead000
clone(strace: Process 4618 attached
child_stack=0x7f48cc6acff8, flags=CLONE_VM|CLONE_SIGHAND|CLONE_THREAD) = 4618
[pid  4618] sched_yield( <unfinished ...>
[pid  4617] sched_setaffinity(4618, 8, [1] <unfinished ...>
[pid  4618] <... sched_yield resumed>)  = 0
[pid  4617] <... sched_setaffinity resumed>) = 0
[pid  4618] sched_yield( <unfinished ...>
[pid  4617] rt_tgsigqueueinfo(4617, 4618, SIGRT_1, {si_signo=SIGINT, si_code=SI_QUEUE, si_pid=0, si_uid=0} <unfinished ...>
[pid  4618] <... sched_yield resumed>)  = 0
[pid  4617] <... rt_tgsigqueueinfo resumed>) = 0
[pid  4618] rt_sigprocmask(SIG_UNBLOCK, [RT_1],  <unfinished ...>
[pid  4617] rt_sigprocmask(SIG_UNBLOCK, [RTMIN],  <unfinished ...>
[pid  4618] <... rt_sigprocmask resumed>NULL, 8) = 0
[pid  4617] <... rt_sigprocmask resumed>NULL, 8) = 0
[pid  4618] --- SIGRT_1 {si_signo=SIGRT_1, si_code=SI_QUEUE, si_pid=0, si_uid=0} ---
[pid  4617] pause( <unfinished ...>
[pid  4618] write(2, "[1] ping\n", 9[1] ping
)   = 9
[pid  4618] rt_tgsigqueueinfo(4617, 4617, SIGRTMIN, {}) = 0
[pid  4617] <... pause resumed>)        = ? ERESTARTNOHAND (To be restarted if no handler)
[pid  4618] rt_sigreturn({mask=[RTMIN]} <unfinished ...>
[pid  4617] --- SIGRTMIN {si_signo=SIGRTMIN, si_code=SI_QUEUE, si_pid=0, si_uid=0} ---
[pid  4618] <... rt_sigreturn resumed>) = 0
[pid  4617] write(1, "[0] pong\n", 9 <unfinished ...>
[pid  4618] pause( <unfinished ...>
[pid  4617] <... write resumed>)        = 9
[pid  4617] rt_tgsigqueueinfo(4617, 4618, SIGRT_1, {}) = 0
[pid  4618] <... pause resumed>)        = ? ERESTARTNOHAND (To be restarted if no handler)
[pid  4617] rt_sigreturn({mask=[RT_1]} <unfinished ...>
[pid  4618] --- SIGRT_1 {si_signo=SIGRT_1, si_code=SI_QUEUE, si_pid=0, si_uid=0} ---
[pid  4617] <... rt_sigreturn resumed>) = -1 EINTR (Interrupted system call)
[pid  4618] write(2, "[1] ping\n", 9 <unfinished ...>
[1] ping
[pid  4617] pause( <unfinished ...>
[pid  4618] <... write resumed>)        = 9
[pid  4618] rt_tgsigqueueinfo(4617, 4617, SIGRTMIN, {} <unfinished ...>
[pid  4617] <... pause resumed>)        = ? ERESTARTNOHAND (To be restarted if no handler)
[pid  4618] <... rt_tgsigqueueinfo resumed>) = 0
[pid  4617] --- SIGRTMIN {si_signo=SIGRTMIN, si_code=SI_QUEUE, si_pid=0, si_uid=0} ---
[pid  4618] rt_sigreturn({mask=[RTMIN]} <unfinished ...>
[pid  4617] write(1, "[0] pong\n", 9 <unfinished ...>
[pid  4618] <... rt_sigreturn resumed>) = -1 EINTR (Interrupted system call)
[pid  4617] <... write resumed>)        = 9
[pid  4618] pause( <unfinished ...>
[pid  4617] exit_group(0)               = ?
[pid  4618] <... pause resumed>)        = ?
[pid  4618] +++ exited with 0 +++
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
   5349      16      33    5398    1516 lock

$ size -Ax lock
lock  :
section               size       addr
.gcc_except_table     0x28   0x200190
.rodata              0x124   0x2001b8
.eh_frame            0x2dc   0x2002e0
.text               0x10bd   0x201000
.data                  0x8   0x203000
.got                   0x8   0x204000
.bss                  0x21   0x205000
.comment              0x12        0x0
Total               0x1528

$ # size on disk in bytes
$ stat -c %s lock
17128

$ # now we produce the smaller binary
$ export RUST_TARGET_PATH=$(pwd)

$ T=x86_64-linux-rtfm

$ xargo build --target $T --example lock --release

$ cp ../target/$T/release/examples/lock .

$ strip -s lock

$ size lock
   text    data     bss     dec     hex filename
   4529       0      33    4562    11d2 lock

$ size -Ax lock
lock  :
section      size       addr
.rodata     0x124   0x200158
.text      0x108d   0x201000
.bss         0x21   0x203000
.comment     0x12        0x0
Total      0x11e4

$ stat -c %s lock
8776
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
prioritization of signal handlers and the `lock` API. Message passing is
implemented using the `rt_sigqueueinfo` system call.

The `timer_create`, `timer_settime` and `clock_gettime(CLOCK_MONOTONIC)` system
calls are used to implement the `schedule` API. Only a single POSIX timer is
used to manage all the `schedule` calls. This timer fires a real-time signal on
timeouts; the handler for that signal is used to "spawn" (`rt_sigqueueinfo`) the
tasks at different priorities.

In single-core mode the framework spawns no additional threads nor does it let
applications spawn them so all software tasks run on a single core and a single
(call) stack.

### Multi-core

In multi-core mode, one "thread" (i.e. a shared-memory process) is spun up
(see `man 2 clone`) for each additional core. Each of these threads is then
pinned to a different physical core using `sched_setaffinity`. The end result is
fully parallel thread execution with no hidden context switching between the
threads (see the `mc-interleaved` example).

Real-time signal handlers are still used to implement software tasks but they
are partitioned across the cores. For example, the first core may use the first
two signal handlers and the second core the next three handlers. The
implementation of the `lock` API doesn't change in this mode and still uses
`rt_sigprocmask`.

In multi-core mode, `spawn` is implemented on top of `rt_tgsigqueueinfo` (note
the TG in the name), which sends a signal to a particular thread rather than to
the whole thread group (i.e. all the threads in our process).

As for the `schedule` API the implementation remains mostly unchanged except
that each core gets its own POSIX timer which fires a different thread-targeted
real-time signal (see `SIGEV_THREAD_ID` in `man 2 timer_create`).

## Notes for `self`

~It should be possible to implement multi-core RTFM by spawning a second thread
(with its own call stack) and pinning it to the second core (using
`sched_setaffinity`). This second thread (core) would have its own set of signal
handlers (software tasks) and one thread would signal the other thread (for
message passing) using the `rt_tgsigqueueinfo` (see `man 2 rt_tgsigqueueinfo`)
system call. See [`rtfm-rt/examples/thread2.rs`] for a proof of concept minus
the `rt_tgsigqueueinfo` part.~ DONE

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
