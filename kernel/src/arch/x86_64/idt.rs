use core::mem::size_of;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    flags: u8,
    offset_mid: u16,
    offset_high: u32,
    reversed: u32,
}

impl IdtEntry {
    const MISSING: Self = Self {
        offset_low: 0,
        selector: 0,
        ist: 0,
        flags: 0,
        offset_mid: 0,
        offset_high: 0,
        reversed: 0,
    };

    fn new(handler: usize) -> Self {
        let address = handler as u64;

        Self {
            offset_low: address as u16,
            selector: 0x08,
            ist: 0,
            flags: 0x8E,
            offset_mid: (address >> 16) as u16,
            offset_high: (address >> 32) as u32,
            reversed: 0,
        }
    }
}

#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

static mut IDT: [IdtEntry; 256] = [IdtEntry::MISSING; 256];

pub fn init() {
    unsafe {
        unsafe extern "C" {
            fn timer_interrupt_stub();
        }

        IDT[0] = IdtEntry::new(crate::arch::x86_64::interrupts::divide_error as *const () as usize);
        IDT[32] = IdtEntry::new(timer_interrupt_stub as *const () as usize);

        let pointer = IdtPointer {
            limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: core::ptr::addr_of!(IDT) as u64,
        };

        core::arch::asm!(
            "lidt [{}]",
            in(reg) &pointer,
            options(readonly, nostack, preserves_flags)
        );
    }

    crate::drivers::serial::println("IDT initalized");
}
