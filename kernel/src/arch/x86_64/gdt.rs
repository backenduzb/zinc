#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

static mut GDT: [u64; 3] = [
    0x0000000000000000, 
    0x00AF9A000000FFFF, 
    0x00AF92000000FFFF, 
];

pub fn init() {
    let pointer = GdtPointer {
        limit: (core::mem::size_of::<[u64; 3]>() - 1) as u16,
        base: unsafe { &raw const GDT as u64 },
    };

    unsafe {
        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &pointer,
            options(readonly, nostack, preserves_flags)
        );

        reload_segments();
    }
}

unsafe fn reload_segments() {
    core::arch::asm!(
        "push 0x08",
        "lea rax, [rip + 2f]",
        "push rax",
        "retfq",

        "2:",
        "mov ax, 0x10",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",

        out("rax") _,
        options(preserves_flags)
    );
}