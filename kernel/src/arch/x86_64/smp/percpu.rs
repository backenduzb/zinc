use core::{sync::atomic::{AtomicBool, Ordering}, usize::MAX};

pub const MAX_CPUS: usize = 16;

pub struct PerCpu {
    pub cpu_id: usize,
    pub apic_id: u32,
    pub online: AtomicBool,
}

impl PerCpu {
    pub const fn new(cpu_id: usize, apic_id: u32) -> Self {
        Self {
            cpu_id,
            apic_id,
            online: AtomicBool::new(false),
        }
    }

    pub fn set_online(&self) {
        self.online.store(true, Ordering::Release);
    }

    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Acquire)
    }
}

static CPU0: PerCpu = PerCpu::new(0, 0);

static CPU_ONLINE: [AtomicBool; MAX_CPUS] =
    [const { AtomicBool::new(false) }; MAX_CPUS];

pub fn bsp() -> &'static PerCpu {
    &CPU0
}

pub fn set_ap_online(cpu_id: usize) {
    if cpu_id >= MAX_CPUS {
        return;
    }

    CPU_ONLINE[cpu_id].store(true, Ordering::Release);

    crate::drivers::serial::print("SMP: CPU ");
    crate::drivers::serial::print_u64(cpu_id as u64);
    crate::drivers::serial::println(" online");
}

pub fn is_online(cpu_id: usize) -> bool {
    if cpu_id >= MAX_CPUS {
        return false;
    }

    CPU_ONLINE[cpu_id].load(Ordering::Acquire)
}

pub fn init_bsp(apic_id: u32) {
    CPU0.online.store(true, Ordering::Release);

    CPU_ONLINE[0].store(true, Ordering::Release);

    crate::drivers::serial::print("SMP: BSP CPU 0 online, APIC ID = ");
    crate::drivers::serial::print_u64(apic_id as u64);
    crate::drivers::serial::println("");
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