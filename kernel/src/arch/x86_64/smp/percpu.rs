use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{arch::x86_64::tss::TaskStateSegment, task::scheduler::Scheduler};

pub const MAX_CPUS: usize = 16;
const IA32_GS_BASE: u32 = 0xC0000101;

const KERNEL_STACK_SIZE: usize = 64 * 1024;

#[repr(align(16))]
struct KernelStack([u8; KERNEL_STACK_SIZE]);

static mut KERNEL_STACKS: [KernelStack; MAX_CPUS] =
    [const { KernelStack([0; KERNEL_STACK_SIZE]) }; MAX_CPUS];

    #[repr(C)]
    pub struct PerCpuGdt {
        pub entries: [u64; 7],
    }
    
    impl PerCpuGdt {
        pub const fn new() -> Self {
            Self {
                entries: [
                    0x0000000000000000, 
                    0x00AF9A000000FFFF, 
                    0x00AF92000000FFFF, 
                    0,                  
                    0,                  
                    0x00AFFA000000FFFF, 
                    0x00AFF2000000FFFF, 
                ],
            }
        }
    }

pub struct PerCpu {
    pub cpu_id: usize,
    pub apic_id: u32,

    pub gdt: UnsafeCell<PerCpuGdt>,
    pub tss: UnsafeCell<TaskStateSegment>,

    pub scheduler: UnsafeCell<Scheduler>,

    online: AtomicBool,
    timer_initialized: AtomicBool,
}

impl PerCpu {
    pub const fn new(cpu_id: usize) -> Self {
        Self {
            cpu_id,
            apic_id: 0,

            gdt: UnsafeCell::new(PerCpuGdt::new()),
            tss: UnsafeCell::new(TaskStateSegment::new()),

            scheduler: UnsafeCell::new(Scheduler::new()),

            online: AtomicBool::new(false),
            timer_initialized: AtomicBool::new(false),
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

    pub fn set_timer_initialized(&self) {
        self.timer_initialized.store(true, Ordering::Release);
    }

    pub fn is_timer_initialized(&self) -> bool {
        self.timer_initialized.load(Ordering::Acquire)
    }

    pub fn current_cpu_id(&self) -> usize {
        self.cpu_id
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

        (*PER_CPUS[0].tss.get()).set_kernel_stack(kernel_stack_top(0));
    }

    let cpu = get(0).expect("BSP PerCpu missing");

    set_gs_base(cpu);

    unsafe {
        let gdt = &mut *cpu.gdt.get();
        let tss = &*cpu.tss.get();

        crate::arch::x86_64::gdt::load_for_cpu(gdt, tss);
    }
    crate::log_tss!("TSS: LTR loaded");

    let rsp0 = unsafe { (*cpu.tss.get()).kernel_stack() };

    let tss = unsafe { &*cpu.tss.get() };

    crate::arch::x86_64::gdt::load_tss(tss);

    crate::log_tss_raw!("CPU 0 TSS RSP0 = ");
    crate::log_smp_raw!("{}", rsp0);
    crate::log_smp!("");

    crate::log_smp_raw!("SMP: BSP CPU 0 online, APIC ID = ");
    crate::log_smp_raw!("{}", apic_id as u64);
    crate::log_smp!("");
}

pub fn init_ap(cpu_id: usize, apic_id: u32) {
    if cpu_id >= MAX_CPUS {
        return;
    }

    unsafe {
        PER_CPUS[cpu_id].set_apic_id(apic_id);
        PER_CPUS[cpu_id].set_online();

        (*PER_CPUS[cpu_id].tss.get()).set_kernel_stack(kernel_stack_top(cpu_id));
    }

    let cpu = get(cpu_id).expect("AP PerCpu missing");

    set_gs_base(cpu);

    unsafe {
        let gdt = &mut *cpu.gdt.get();
        let tss = &*cpu.tss.get();

        crate::arch::x86_64::gdt::load_for_cpu(gdt, tss);
    }
    crate::log_tss!("TSS: LTR loaded");

    let cpu = get(cpu_id).expect("AP PerCpu missing");

    let rsp0 = unsafe { (*cpu.tss.get()).kernel_stack() };

    crate::log_smp_raw!("CPU ");
    crate::log_smp_raw!("{}", cpu_id as u64);
    crate::log_tss_raw!(" TSS RSP0 = ");
    crate::log_smp_raw!("{}", rsp0);
    crate::log_smp!("");

    crate::log_smp_raw!("SMP: CPU ");
    crate::log_smp_raw!("{}", cpu_id as u64);
    crate::log_smp_raw!(" online, APIC ID = ");
    crate::log_smp_raw!("{}", apic_id as u64);
    crate::log_smp!("");
}
pub fn is_online(cpu_id: usize) -> bool {
    match get(cpu_id) {
        Some(cpu) => cpu.is_online(),
        None => false,
    }
}

pub fn is_timer_initialized(cpu_id: usize) -> bool {
    match get(cpu_id) {
        Some(cpu) => cpu.is_timer_initialized(),
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
    let gs = unsafe { crate::arch::x86_64::io::rdmsr(IA32_GS_BASE) };

    if gs == 0 {
        panic!("SMP: GS.base is not initialized");
    }

    unsafe { &*(gs as *const PerCpu) }
}

pub fn set_gs_base(cpu: &'static PerCpu) {
    unsafe {
        crate::arch::x86_64::io::wrmsr(IA32_GS_BASE, cpu as *const PerCpu as u64);
    }
}

pub fn kernel_stack_top(cpu_id: usize) -> u64 {
    unsafe {
        let stack = &KERNEL_STACKS[cpu_id];

        stack.0.as_ptr() as u64 + KERNEL_STACK_SIZE as u64
    }
}

pub fn current_scheduler() -> &'static mut Scheduler {
    let cpu = current();

    unsafe {
        &mut *cpu.scheduler.get()
    }
}

pub fn current_cpu_id() -> usize {
    current().cpu_id
}