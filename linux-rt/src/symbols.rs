use core::hint;

// One true entry point
// 1. clear the frame point
// 2. pass the top of the stack to `start`
// 3. align the stack to 16 bytes
#[cfg(target_arch = "x86_64")]
global_asm!(
    r#"
  .global _start
  .section .text._start
_start:
  xor %rbp,%rbp
  mov %rsp, %rdi
  andq $-16, %rsp
  call start
"#
);

// we don't link to `libc.a` and `compiler-builtins` doesn't provide these symbols on some targets
// so we need to provide them ourselves; they need to be written in assembly or we'll end with
// infinite recursion. The assembly used is the output of compiling these two Rust functions with
// `opt-level=z`
//
// #[no_mangle]
// unsafe extern "C" fn memcpy(mut dest: *mut u8, mut src: *const u8, count: usize) {
//     for _ in 0..count {
//         dest.write_volatile(src.read_volatile());
//         src = src.add(1);
//         dest = dest.add(1);
//     }
// }
//
// #[no_mangle]
// unsafe extern "C" fn memset(mut dest: *mut u8, ch: i32, count: usize) {
//     for _ in 0..count {
//         dest.write_volatile(ch as u8);
//         dest = dest.add(1);
//     }
// }

#[cfg(target_arch = "x86_64")]
global_asm!(
    r#"
  .global memcpy
  .section .text.memcpy
memcpy:
  movq %rdi, %rax
  xorl %edi, %edi
  jmp  2f
1:movb (%rsi,%rdi), %cl
  movb %cl, (%rax,%rdi)
  incq %rdi
2:cmpq %rdi, %rdx
  jne  1b
  addq %rdi, %rax
  retq
"#
);

#[cfg(target_arch = "x86_64")]
global_asm!(
    r#"
  .global memset
  .section .text.memset
memset:
  xorl %eax, %eax
  jmp  2f
1:movb %sil, (%rdi,%rax)
  incq %rax
2:cmpq %rax, %rdx
  jne  1b
"#
);

// `__restorer` is used to return from signal handlers and must call the RT_SIGRETURN system call.
// `__restorer` can't modify the stack so it must be written in assembly
#[cfg(target_arch = "x86_64")]
global_asm!(
    r#"
  .global __restorer
  .section .text.__restorer
__restorer:
  mov $15, %rax
  syscall
"#
);

// `core` was compiled with `-C panic=unwind` so it contains undefined references to these symbols
#[allow(unused_attributes)]
#[allow(non_snake_case)]
#[no_mangle]
fn _Unwind_Resume() {
    unsafe { hint::unreachable_unchecked() }
}

#[allow(unused_attributes)]
#[allow(non_snake_case)]
#[no_mangle]
fn rust_eh_personality() {
    unsafe { hint::unreachable_unchecked() }
}
