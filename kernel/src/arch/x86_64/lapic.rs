use core::sync::atomic::{AtomicU64, Ordering};

const LAPIC_DEFAULT_BASE: u64 = 0xFEE0_0000;

const REG_ID: u64 = 0x020;
const REG_VERSION: u64 = 0x030;
const REG_EOI: u64 = 0x0B0;
const REG_SVR: u64 = 0x0F0;

const REG_ICR_LOW: u64 = 0x300;
const REG_ICR_HIGH: u64 = 0x310;

pub const LAPIC_SIZE: u64 = 0x1000;

const SVR_ENABLE: u32 = 1 << 8;
const SPURIOUS_VECTOR: u32 = 0xFF;

const ICR_DELIVERY_INIT: u32 = 0b101 << 8;
const ICR_DELIVERY_STARTUP: u32 = 0b110 << 8;

const ICR_LEVEL_ASSERT: u32 = 1 << 14;
const ICR_TRIGGER_LEVEL: u32 = 1 << 15;

const ICR_DELIVERY_STATUS: u32 = 1 << 12;

pub static LAPIC_TICKS: AtomicU64 = AtomicU64::new(0); 

static mut BASE: u64 = LAPIC_DEFAULT_BASE;

const REG_LVT_TIMER: u64 = 0x320;
const REG_INITIAL_COUNT: u64 = 0x380;
const REG_CURRENT_COUNT: u64 = 0x390;
const REG_DIVIDE_CONFIG: u64 = 0x3E0;

const LVT_PERIODIC: u32 = 1 << 17;

pub const TIMER_VECTOR: u8 = 48;

unsafe fn read(offset: u64) -> u32 {
    core::ptr::read_volatile((BASE + offset) as *const u32)
}

unsafe fn write(offset: u64, value: u32) {
    core::ptr::write_volatile((BASE + offset) as *mut u32, value);
}

fn wait_icr_idle() {
    for _ in 0..1_000_000 {
        let value = unsafe { read(REG_ICR_LOW) };

        if value & ICR_DELIVERY_STATUS == 0 {
            return;
        }

        core::hint::spin_loop();
    }

    crate::drivers::serial::println("LAPIC: ICR delivery timeout");
}

pub fn phys_to_virt(phys: u64) -> u64 {
    crate::boot::limine::physical_to_virtual(phys).unwrap_or(phys)
}

pub fn init(base: u64) {
    crate::drivers::serial::println("LAPIC: initializing");

    let virt = phys_to_virt(base);

    crate::drivers::serial::print("LAPIC physical base: 0x");
    crate::drivers::serial::print_hex_u64(base);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("LAPIC virtual base: 0x");
    crate::drivers::serial::print_hex_u64(virt);
    crate::drivers::serial::println("");

    unsafe {
        BASE = virt;

        let svr = read(REG_SVR);

        write(REG_SVR, (svr & 0xFF) | SVR_ENABLE | SPURIOUS_VECTOR);
    }

    crate::drivers::serial::println("LAPIC: enabled");

    crate::drivers::serial::print("LAPIC ID: ");
    crate::drivers::serial::print_u64(id() as u64);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("LAPIC version: 0x");
    crate::drivers::serial::print_hex_u32(version());
    crate::drivers::serial::println("");
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

pub fn send_init_ipi(apic_id: u32) {
    wait_icr_idle();

    unsafe {
        write(REG_ICR_HIGH, apic_id << 24);

        write(
            REG_ICR_LOW,
            ICR_DELIVERY_INIT | ICR_LEVEL_ASSERT | ICR_TRIGGER_LEVEL,
        );
    }

    wait_icr_idle();

    crate::drivers::serial::print("LAPIC: INIT -> APIC ");
    crate::drivers::serial::print_u64(apic_id as u64);
    crate::drivers::serial::println("");
}

pub fn deassert_init() {
    wait_icr_idle();

    unsafe {
        write(REG_ICR_HIGH, 0);

        write(REG_ICR_LOW, ICR_TRIGGER_LEVEL);
    }

    wait_icr_idle();

    crate::drivers::serial::println("LAPIC: INIT deassert");
}

pub fn send_startup_ipi(apic_id: u32, vector: u8) {
    wait_icr_idle();

    unsafe {
        write(REG_ICR_HIGH, apic_id << 24);

        write(REG_ICR_LOW, ICR_DELIVERY_STARTUP | vector as u32);
    }

    wait_icr_idle();

    crate::drivers::serial::print("LAPIC: SIPI -> APIC ");
    crate::drivers::serial::print_u64(apic_id as u64);

    crate::drivers::serial::print(" vector=0x");
    crate::drivers::serial::print_hex_u32(vector as u32);

    crate::drivers::serial::println("");
}

fn delay() {
    for _ in 0..100_000 {
        core::hint::spin_loop();
    }
}

pub fn start_ap(apic_id: u32, vector: u8) {
    crate::drivers::serial::print("LAPIC: starting AP ");
    crate::drivers::serial::print_u64(apic_id as u64);
    crate::drivers::serial::println("");

    send_init_ipi(apic_id);
    delay();

    deassert_init();
    delay();

    send_startup_ipi(apic_id, vector);
    delay();

    send_startup_ipi(apic_id, vector);
    delay();
}

pub fn timer_init() {
    unsafe {
        write(REG_DIVIDE_CONFIG, 0x3);
        write(REG_LVT_TIMER, TIMER_VECTOR as u32 | LVT_PERIODIC);
        write(REG_INITIAL_COUNT, 0);

        let lvt = read(REG_LVT_TIMER);
        let divide = read(REG_DIVIDE_CONFIG);

        crate::drivers::serial::print("LAPIC LVT TIMER: 0x");
        crate::drivers::serial::print_hex_u32(lvt);
        crate::drivers::serial::println("");

        crate::drivers::serial::print("LAPIC DIVIDE: 0x");
        crate::drivers::serial::print_hex_u32(divide);
        crate::drivers::serial::println("");
    }

    crate::drivers::serial::println("LAPIC: timer initialized");
}

pub fn timer_start(count: u32) {
    unsafe {
        write(REG_INITIAL_COUNT, count);

        let current = read(REG_CURRENT_COUNT);

        crate::drivers::serial::print("LAPIC initial count: ");
        crate::drivers::serial::print_u64(count as u64);
        crate::drivers::serial::println("");

        crate::drivers::serial::print("LAPIC current count: ");
        crate::drivers::serial::print_u64(current as u64);
        crate::drivers::serial::println("");
    }

    crate::drivers::serial::println("LAPIC: timer started");
}

pub fn timer_debug_count() {
    unsafe {
        let current = read(REG_CURRENT_COUNT);

        crate::drivers::serial::print("LAPIC current count now: ");
        crate::drivers::serial::print_u64(current as u64);
        crate::drivers::serial::println("");
    }
}

pub fn ticks() -> u64 {
    LAPIC_TICKS.load(Ordering::Relaxed)
}

#[inline]
pub fn tick() -> u64 {
    LAPIC_TICKS.fetch_add(1, Ordering::Relaxed) + 1
}