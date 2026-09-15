use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86_64::cpu;

pub const MAX_CPUS: usize = 16;
const IA32_GS_BASE: u32 = 0xC0000101;

pub struct PerCpu {
    pub cpu_id: usize,
    pub apic_id: u32,
    online: AtomicBool,
}

impl PerCpu {
    pub const fn new(cpu_id: usize) -> Self {
        Self {
            cpu_id,
            apic_id: 0,
            online: AtomicBool::new(false),
        }
    }

    pub fn set_apic_id(&mut self, apic_id: u32) {
        self.apic_id = apic_id;
    }

    pub fn set_online(&self) {
        self.online.store(true, Ordering::Release);
    }

    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Acquire)
    }
}

static mut PER_CPUS: [PerCpu; MAX_CPUS] = [const { PerCpu::new(0) }; MAX_CPUS];

pub fn init() {
    unsafe {
        for cpu_id in 0..MAX_CPUS {
            PER_CPUS[cpu_id] = PerCpu::new(cpu_id);
        }
    }
}

pub fn get(cpu_id: usize) -> Option<&'static PerCpu> {
    if cpu_id >= MAX_CPUS {
        return None;
    }

    unsafe { Some(&PER_CPUS[cpu_id]) }
}

pub fn bsp() -> &'static PerCpu {
    get(0).expect("BSP PerCpu missing")
}

pub fn init_bsp(apic_id: u32) {
    unsafe {
        PER_CPUS[0].set_apic_id(apic_id);
        PER_CPUS[0].set_online();
    }

    let cpu = get(0).expect("BSP PerCpu missing");

    set_gs_base(cpu);

    crate::drivers::serial::print(
        "SMP: BSP CPU 0 online, APIC ID = ",
    );
    crate::drivers::serial::print_u64(apic_id as u64);
    crate::drivers::serial::println("");
}

pub fn init_ap(cpu_id: usize, apic_id: u32) {
    if cpu_id >= MAX_CPUS {
        return;
    }

    unsafe {
        PER_CPUS[cpu_id].set_apic_id(apic_id);
        PER_CPUS[cpu_id].set_online();
    }

    let cpu = get(cpu_id).expect("AP PerCpu missing");

    set_gs_base(cpu);

    crate::drivers::serial::print("SMP: CPU ");
    crate::drivers::serial::print_u64(cpu_id as u64);
    crate::drivers::serial::print(" online, APIC ID = ");
    crate::drivers::serial::print_u64(apic_id as u64);
    crate::drivers::serial::println("");
}
pub fn is_online(cpu_id: usize) -> bool {
    match get(cpu_id) {
        Some(cpu) => cpu.is_online(),
        None => false,
    }
}

pub fn online_count() -> usize {
    let mut count = 0;

    for cpu_id in 0..MAX_CPUS {
        if is_online(cpu_id) {
            count += 1;
        }
    }

    count
}

pub fn current() -> &'static PerCpu {
    let gs = unsafe {
        crate::arch::x86_64::io::rdmsr(IA32_GS_BASE)
    };

    if gs == 0 {
        panic!("SMP: GS.base is not initialized");
    }

    unsafe {
        &*(gs as *const PerCpu)
    }
}

pub fn set_gs_base(cpu: &'static PerCpu) {
    unsafe {
        crate::arch::x86_64::io::wrmsr(
            IA32_GS_BASE,
            cpu as *const PerCpu as u64,
        );
    }
}

