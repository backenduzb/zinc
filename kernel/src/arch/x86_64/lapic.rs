const LAPIC_DEFAULT_BASE: u64 = 0xFEE0_0000;

const REG_ID: u64 = 0x020;
const REG_VERSION: u64 = 0x030;
const REG_EOI: u64 = 0x0B0;
const REG_SVR: u64 = 0x0F0;
const REG_ICR_LOW: u64 = 0x300;
const REG_ICR_HIGH: u64 = 0x310;

pub const LAPIC_SIZE: u64 = 0x1000;

static mut BASE: u64 = LAPIC_DEFAULT_BASE;

#[inline]
unsafe fn read(offset: u64) -> u32 {
    let addr = (BASE + offset) as *const u32;
    core::ptr::read_volatile(addr)
}

#[inline]
unsafe fn write(offset: u64, value: u32) {
    let addr = (BASE + offset) as *mut u32;
    core::ptr::write_volatile(addr, value);
}

pub fn init(base: u64) {
    crate::drivers::serial::println("LAPIC: initializing");

    crate::drivers::serial::print("LAPIC: physical base = 0x");
    crate::drivers::serial::print_hex_u64(base);
    crate::drivers::serial::println("");

    let virt = phys_to_virt(base);

    crate::drivers::serial::print("LAPIC: virtual base = 0x");
    crate::drivers::serial::print_hex_u64(virt);
    crate::drivers::serial::println("");

    unsafe {
        BASE = virt;
    }

    crate::drivers::serial::println("LAPIC: MMIO base configured");
}

pub fn id() -> u32 {
    unsafe { read(REG_ID) >> 24 }
}

pub fn version() -> u32 {
    unsafe { read(REG_VERSION) }
}

pub fn eoi() {
    unsafe {
        write(REG_EOI, 0);
    }
}

pub fn phys_to_virt(phys: u64) -> u64 {
    crate::boot::limine::physical_to_virtual(phys).unwrap_or(phys)
}
