#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nostack, preserves_flags)
        );
    }
}

#[inline]
pub unsafe fn io_wait() {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x80u16,
            in("al") 0u8,
            options(nostack, preserves_flags)
        );
    }
}

#[inline]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}

#[inline]
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let lo = value as u32;
    let hi = (value >> 32) as u32;
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") lo,
            in("edx") hi,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;

    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nostack, preserves_flags)
        );
    }

    value
}

#[inline]
pub unsafe fn outl(port: u16, value: u32) {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
            options(nostack, preserves_flags)
        );
    }
}

#[inline]
pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;

    unsafe {
        core::arch::asm!(
            "in eax, dx",
            in("dx") port,
            out("eax") value,
            options(nostack, preserves_flags)
        );
    }

    value
}
