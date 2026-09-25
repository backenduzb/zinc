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

pub const MAX_AP_CPUS: usize = 16;
static AP_TICKS: [AtomicU64; MAX_AP_CPUS] = [const { AtomicU64::new(0) }; MAX_AP_CPUS];

pub const TIMER_INITIAL_COUNT: u32 = 10_000_000;

static mut BASE: u64 = LAPIC_DEFAULT_BASE;

const REG_LVT_TIMER: u64 = 0x320;
const REG_LVT_LINT0: u64 = 0x350;
const REG_LVT_LINT1: u64 = 0x360;
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

    crate::log_smp!("LAPIC: ICR delivery timeout");
}

pub fn phys_to_virt(phys: u64) -> u64 {
    crate::boot::limine::physical_to_virtual(phys).unwrap_or(phys)
}

pub fn init(base: u64) {
    crate::log_smp!("LAPIC: initializing");

    let virt = phys_to_virt(base);

    crate::log_smp_raw!("LAPIC physical base: 0x");
    crate::log_smp_raw!("{:016X}", base);
    crate::log_smp!("");

    crate::log_smp_raw!("LAPIC virtual base: 0x");
    crate::log_smp_raw!("{:016X}", virt);
    crate::log_smp!("");

    unsafe {
        BASE = virt;

        let svr = read(REG_SVR);

        write(REG_SVR, (svr & 0xFF) | SVR_ENABLE | SPURIOUS_VECTOR);

        write(REG_LVT_LINT0, 0x700);
        write(REG_LVT_LINT1, 0x400); 
    }


    crate::log_smp!("LAPIC: enabled");

    crate::log_smp_raw!("LAPIC ID: ");
    crate::log_smp_raw!("{}", id() as u64);
    crate::log_smp!("");

    crate::log_smp_raw!("LAPIC version: 0x");
    crate::log_smp_raw!("{:08X}", version());
    crate::log_smp!("");
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

    crate::log_smp_raw!("LAPIC: INIT -> APIC ");
    crate::log_smp_raw!("{}", apic_id as u64);
    crate::log_smp!("");
}

pub fn deassert_init() {
    wait_icr_idle();

    unsafe {
        write(REG_ICR_HIGH, 0);

        write(REG_ICR_LOW, ICR_TRIGGER_LEVEL);
    }

    wait_icr_idle();

    crate::log_smp!("LAPIC: INIT deassert");
}

pub fn send_startup_ipi(apic_id: u32, vector: u8) {
    wait_icr_idle();

    unsafe {
        write(REG_ICR_HIGH, apic_id << 24);

        write(REG_ICR_LOW, ICR_DELIVERY_STARTUP | vector as u32);
    }

    wait_icr_idle();

    crate::log_smp_raw!("LAPIC: SIPI -> APIC ");
    crate::log_smp_raw!("{}", apic_id as u64);

    crate::log_smp_raw!(" vector=0x");
    crate::log_smp_raw!("{:08X}", vector as u32);

    crate::log_smp!("");
}

fn delay() {
    for _ in 0..100_000 {
        core::hint::spin_loop();
    }
}

pub fn start_ap(apic_id: u32, vector: u8) {
    crate::log_smp_raw!("LAPIC: starting AP ");
    crate::log_smp_raw!("{}", apic_id as u64);
    crate::log_smp!("");

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

        crate::log_smp_raw!("LAPIC LVT TIMER: 0x");
        crate::log_smp_raw!("{:08X}", lvt);
        crate::log_smp!("");

        crate::log_smp_raw!("LAPIC DIVIDE: 0x");
        crate::log_smp_raw!("{:08X}", divide);
        crate::log_smp!("");
    }

    crate::log_smp!("LAPIC: timer initialized");
}

pub fn timer_start(count: u32) {
    unsafe {
        write(REG_INITIAL_COUNT, count);

        let current = read(REG_CURRENT_COUNT);

        crate::log_smp_raw!("LAPIC initial count: ");
        crate::log_smp_raw!("{}", count as u64);
        crate::log_smp!("");

        crate::log_smp_raw!("LAPIC current count: ");
        crate::log_smp_raw!("{}", current as u64);
        crate::log_smp!("");
    }

    crate::log_smp!("LAPIC: timer started");
}

pub fn timer_debug_count() {
    unsafe {
        let current = read(REG_CURRENT_COUNT);

        crate::log_smp_raw!("LAPIC current count now: ");
        crate::log_smp_raw!("{}", current as u64);
        crate::log_smp!("");
    }
}

pub fn ticks() -> u64 {
    LAPIC_TICKS.load(Ordering::Relaxed)
}

#[inline]
pub fn tick() -> u64 {
    LAPIC_TICKS.fetch_add(1, Ordering::Relaxed) + 1
}

#[inline]
pub fn ap_tick(cpu_id: usize) -> u64 {
    if cpu_id >= MAX_AP_CPUS {
        return 0;
    }
    AP_TICKS[cpu_id].fetch_add(1, Ordering::Relaxed) + 1
}

#[inline]
pub fn ap_ticks(cpu_id: usize) -> u64 {
    if cpu_id >= MAX_AP_CPUS {
        return 0;
    }
    AP_TICKS[cpu_id].load(Ordering::Relaxed)
}

pub fn init_ap() {
    let apic_id = id();
    unsafe {
        let svr = read(REG_SVR);
        write(REG_SVR, (svr & 0xFF) | SVR_ENABLE | SPURIOUS_VECTOR);
        write(REG_LVT_LINT0, 0x700);
        write(REG_LVT_LINT1, 0x400);
    }
    crate::log_smp_raw!("AP LAPIC: enabled, ID = ");
    crate::log_smp_raw!("{}", apic_id as u64);
    crate::log_smp!("");
}

pub fn timer_init_ap() {
    unsafe {
        write(REG_DIVIDE_CONFIG, 0x3);
        write(REG_LVT_TIMER, TIMER_VECTOR as u32 | LVT_PERIODIC);
        write(REG_INITIAL_COUNT, 0);
    }
}

pub fn timer_start_ap(count: u32) {
    unsafe {
        write(REG_INITIAL_COUNT, count);
    }
    crate::log_smp_raw!("AP LAPIC: timer started, count = ");
    crate::log_smp_raw!("{}", count as u64);
    crate::log_smp!("");
}