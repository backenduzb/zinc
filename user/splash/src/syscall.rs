use core::arch::asm;

pub const SYS_YIELD: u64 = 0;
pub const SYS_EXIT: u64 = 1;
pub const SYS_GFX_INFO: u64 = 8;
pub const SYS_GFX_CLEAR: u64 = 9;
pub const SYS_GFX_RECT: u64 = 10;
pub const SYS_CLOCK_UPTIME: u64 = 12;

#[inline(always)]
pub fn syscall0(number: u64) -> u64 {
    let result: u64;

    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") number => result,
        );
    }

    result
}

#[inline(always)]
pub fn syscall1(number: u64, arg1: u64) -> u64 {
    let result: u64;

    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") number => result,
            in("rdi") arg1,
        );
    }

    result
}

#[inline(always)]
pub fn syscall2(number: u64, arg1: u64, arg2: u64) -> u64 {
    let result: u64;

    unsafe {
        asm!(
            "int 0x80",
            inlateout("rax") number => result,
            in("rdi") arg1,
            in("rsi") arg2,
        );
    }

    result
}

#[inline(always)]
pub fn syscall3(number: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let result: u64;

    unsafe {
        core::arch::asm!(
            "int 0x80",
            inlateout("rax") number => result,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
        );
    }

    result
}

#[inline(always)]
pub fn uptime_ms() -> u64 {
    syscall0(SYS_CLOCK_UPTIME)
}

pub fn yield_now() {
    let _ = syscall0(SYS_YIELD);
}

pub fn exit(code: u64) -> ! {
    let _ = syscall1(SYS_EXIT, code);

    loop {
        core::hint::spin_loop();
    }
}
