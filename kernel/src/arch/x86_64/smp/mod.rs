use crate::drivers::serial::println;

pub mod acpi;
pub mod trampoline;
pub mod percpu;

#[derive(Clone, Copy)]
pub struct SmpCpu {
    pub processor_id: u8,
    pub apic_id: u32,
    pub enabled: bool,
    pub started: bool,
}
pub const MAX_CPUS: usize = 64;

static mut CPUS: [SmpCpu; MAX_CPUS] = [SmpCpu {
    processor_id: 0,
    apic_id: 0,
    enabled: false,
    started: false,
}; MAX_CPUS];
static mut CPU_COUNT: usize = 0;

pub fn register_cpu(processor_id: u8, apic_id: u32, enabled: bool) {
    unsafe {
        if CPU_COUNT >= MAX_CPUS {
            return;
        }

        CPUS[CPU_COUNT] = SmpCpu {
            processor_id,
            apic_id,
            enabled,
            started: false,
        };
        
        CPU_COUNT += 1;
    }
}

pub fn cpu_count() -> usize {
    unsafe { CPU_COUNT }
}

pub fn init() {
    crate::arch::x86_64::smp::percpu::init();
    crate::log_smp!("SMP: initializing ACPI");
    acpi::init();
}