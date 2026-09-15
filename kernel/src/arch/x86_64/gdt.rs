use crate::arch::x86_64::tss::TaskStateSegment;
use crate::arch::x86_64::smp::percpu::PerCpuGdt;

#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

const GDT_KERNEL_CODE: u16 = 0x08;
const GDT_KERNEL_DATA: u16 = 0x10;
const GDT_TSS: u16 = 0x18;

static mut GDT: [u64; 5] = [
    0x0000000000000000,
    0x00AF9A000000FFFF,
    0x00AF92000000FFFF,
    0x0000000000000000,
    0x0000000000000000,
];

pub fn init() {
    let pointer = GdtPointer {
        limit: (core::mem::size_of::<[u64; 5]>() - 1) as u16,
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
        "push {code}",
        "lea rax, [rip + 2f]",
        "push rax",
        "retfq",

        "2:",
        "mov ax, {data}",
        "mov ds, ax",
        "mov es, ax",
        "mov ss, ax",

        code = const GDT_KERNEL_CODE,
        data = const GDT_KERNEL_DATA,

        out("rax") _,
        options(preserves_flags)
    );
}

pub fn load_tss(tss: &'static crate::arch::x86_64::tss::TaskStateSegment) {
    let base = tss as *const _ as u64;
    let limit = (core::mem::size_of::<
        crate::arch::x86_64::tss::TaskStateSegment
    >() - 1) as u32;

    let low =
        (limit as u64)
        | ((base & 0xFFFF) << 16)
        | (((base >> 16) & 0xFF) << 32)
        | (0x89u64 << 40)
        | (((limit >> 16) as u64 & 0xF) << 48)
        | (((base >> 24) & 0xFF) << 56);

    let high = base >> 32;

    unsafe {
        GDT[3] = low;
        GDT[4] = high;

        let pointer = GdtPointer {
            limit: (core::mem::size_of::<[u64; 5]>() - 1) as u16,
            base: &raw const GDT as u64,
        };

        core::arch::asm!(
            "lgdt [{}]",
            in(reg) &pointer,
            options(readonly, nostack, preserves_flags)
        );

        let selector: u16 = GDT_TSS;

        core::arch::asm!(
            "ltr {selector:x}",
            selector = in(reg) selector,
            options(nostack, preserves_flags)
        );
    }
}

pub unsafe fn load_for_cpu(
    gdt: &'static mut PerCpuGdt,
    tss: &'static TaskStateSegment,
) {
    let base = tss as *const _ as u64;
    let limit = (core::mem::size_of::<TaskStateSegment>() - 1) as u32;

    let low =
        (limit as u64)
        | ((base & 0xFFFF) << 16)
        | (((base >> 16) & 0xFF) << 32)
        | (0x89u64 << 40)
        | (((limit >> 16) as u64 & 0xF) << 48)
        | (((base >> 24) & 0xFF) << 56);

    let high = base >> 32;

    gdt.entries[3] = low;
    gdt.entries[4] = high;

    let pointer = GdtPointer {
        limit: (core::mem::size_of::<PerCpuGdt>() - 1) as u16,
        base: gdt as *const _ as u64,
    };

    core::arch::asm!(
        "lgdt [{}]",
        in(reg) &pointer,
        options(readonly, nostack, preserves_flags)
    );

    reload_segments();

    let selector: u16 = 0x18;

    core::arch::asm!(
        "ltr {selector:x}",
        selector = in(reg) selector,
        options(nostack, preserves_flags)
    );
}