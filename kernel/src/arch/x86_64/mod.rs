pub mod context;
pub mod cpu;
pub mod gdt;
pub mod idt;
pub mod interrupts;
pub mod io;
pub mod paging;

pub fn init() {
    gdt::init();
    idt::init();

    disable_lapic_for_pic();

    crate::drivers::timer::init();

    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::serial::println("x86_64 architecture registered");
}

// LAPIC yoqilgan bo'lsa 8259 PIC interruptlari CPU ga yetib bormaydi.
// Shuning uchun PIC ishlatish uchun LAPIC ni o'chiramiz (MSR 0x1B, bit 11).
fn disable_lapic_for_pic() {
    unsafe {
        let apic_msr = crate::arch::x86_64::io::rdmsr(0x1B);
        if (apic_msr & (1u64 << 11)) != 0 {
            crate::arch::x86_64::io::wrmsr(0x1B, apic_msr & !(1u64 << 11));
            crate::drivers::serial::println("LAPIC disabled, using PIC");
        }
    }
}
