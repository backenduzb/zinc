use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86_64::smp::percpu::MAX_CPUS;

const TRAMPOLINE_PHYS: u64 = 0x8000;

const CR3_OFFSET: usize = 0x100;
const STACK_OFFSET: usize = 0x108;
const ENTRY_OFFSET: usize = 0x110;

const TRAMPOLINE_SIZE: usize = 0x120;

static AP_STARTED: [AtomicBool; MAX_CPUS] = [const { AtomicBool::new(false) }; MAX_CPUS];

static TRAMPOLINE: &[u8] = include_bytes!("../../../../../boot/ap/build/trampoline.bin");

pub fn install(cr3: u64, stack: u64, entry: u64) {
    if TRAMPOLINE.len() < TRAMPOLINE_SIZE {
        crate::log_smp!("SMP: trampoline too small");
        return;
    }

    let dest_phys = TRAMPOLINE_PHYS;

    let dest_virt = crate::boot::limine::physical_to_virtual(dest_phys)
        .expect("SMP: failed to translate trampoline physical address");

    crate::log_smp_raw!("SMP: trampoline physical: 0x");
    crate::log_smp_raw!("{:016X}", dest_phys);
    crate::log_smp!("");

    crate::log_smp_raw!("SMP: trampoline virtual: 0x");
    crate::log_smp_raw!("{:016X}", dest_virt);
    crate::log_smp!("");

    unsafe {
        let dst = dest_virt as *mut u8;

        core::ptr::copy_nonoverlapping(TRAMPOLINE.as_ptr(), dst, TRAMPOLINE.len());

        core::ptr::write_unaligned(dst.add(CR3_OFFSET) as *mut u64, cr3);

        core::ptr::write_unaligned(dst.add(STACK_OFFSET) as *mut u64, stack);

        core::ptr::write_unaligned(dst.add(ENTRY_OFFSET) as *mut u64, entry);
    }

    crate::log_smp!("SMP: AP trampoline installed");

    crate::log_cr3_raw!("SMP: trampoline CR3: 0x");
    crate::log_smp_raw!("{:016X}", cr3);
    crate::log_smp!("");

    crate::log_smp_raw!("SMP: trampoline stack: 0x");
    crate::log_smp_raw!("{:016X}", stack);
    crate::log_smp!("");

    crate::log_smp_raw!("SMP: trampoline entry: 0x");
    crate::log_smp_raw!("{:016X}", entry);
    crate::log_smp!("");
}

pub fn is_ap_started(cpu_id: usize) -> bool {
    if cpu_id >= MAX_CPUS {
        return false;
    }
    AP_STARTED[cpu_id].load(Ordering::Acquire)
}

pub fn reset_ap_state(cpu_id: usize) {
    if cpu_id >= MAX_CPUS {
        return;
    }

    AP_STARTED[cpu_id].store(false, Ordering::Release);
}

#[unsafe(no_mangle)]
pub extern "C" fn ap_main() -> ! {
    unsafe {
        core::arch::asm!("cli");
    }

    let apic_id = crate::arch::x86_64::lapic::id();
    let cpu_id = apic_id as usize;

    crate::log_smp_raw!("AP: entered trampoline, LAPIC ID = ");
    crate::log_smp_raw!("{}", apic_id as u64);
    crate::log_smp!("");

    if cpu_id < MAX_CPUS {
        AP_STARTED[cpu_id].store(true, Ordering::Release);

        crate::arch::x86_64::smp::percpu::init_ap(cpu_id, apic_id);
    }

    let cpu = crate::arch::x86_64::smp::percpu::current();

    crate::log_smp_raw!("AP: PerCpu CPU ID = ");
    crate::log_smp_raw!("{}", cpu.cpu_id as u64);
    crate::log_smp!("");

    crate::log_smp_raw!("AP: PerCpu APIC ID = ");
    crate::log_smp_raw!("{}", cpu.apic_id as u64);
    crate::log_smp!("");

    crate::log_smp_raw!("AP: GS CPU ID = ");
    crate::log_smp_raw!("{}", cpu.cpu_id as u64);
    crate::log_smp!("");

    crate::log_smp_raw!("AP: GS APIC ID = ");
    crate::log_smp_raw!("{}", cpu.apic_id as u64);
    crate::log_smp!("");

    crate::log_smp_raw!("SMP: GS current CPU = ");
    crate::log_smp_raw!("{}", cpu.cpu_id as u64);
    crate::log_smp!("");

    /*
     * Load AP-local IDT.
     */
    crate::arch::x86_64::idt::load_ap();

    /*
     * Enable local APIC on this CPU.
     */
    crate::arch::x86_64::lapic::init_ap();

    /*
     * Initialize and start AP-local LAPIC timer.
     */
    crate::arch::x86_64::lapic::timer_init_ap();

    crate::arch::x86_64::lapic::timer_start_ap(
        crate::arch::x86_64::lapic::TIMER_INITIAL_COUNT
    );

    let cpu = crate::arch::x86_64::smp::percpu::current();
    cpu.set_timer_initialized();

    crate::log_smp_raw!("AP LAPIC: timer initialized, CPU=");
    crate::log_smp_raw!("{}", cpu.cpu_id as u64);
    crate::log_smp!("");

    /*
     * Initialize scheduler and test thread for this AP.
     */
    if cpu_id != 0 && cpu_id < crate::arch::x86_64::smp::percpu::MAX_CPUS {
        use crate::task::test::{
            AP_BETA_ID,
            AP_GAMMA_ID,
            AP_SILF_ID,
        };
        use crate::task::thread::Thread;

        /*
         * Every CPU has its own scheduler-local idle thread.
         */
        unsafe {
            crate::task::SCHEDULERS[cpu_id].init_idle(cpu_id);
        }

        crate::log_sched_raw!("AP Scheduler initialized: CPU=");
        crate::log_smp_raw!("{}", cpu_id as u64);
        crate::log_smp!("");

        let (thread, name, thread_id) = match cpu_id {
            1 => (
                Thread::new(
                    AP_BETA_ID,
                    cpu_id,
                    0,
                    crate::task::test::ap_beta,
                ),
                "AP_BETA",
                AP_BETA_ID,
            ),

            2 => (
                Thread::new(
                    AP_SILF_ID,
                    cpu_id,
                    0,
                    crate::task::test::ap_silf,
                ),
                "AP_SILF",
                AP_SILF_ID,
            ),

            3 => (
                Thread::new(
                    AP_GAMMA_ID,
                    cpu_id,
                    0,
                    crate::task::test::ap_gamma,
                ),
                "AP_GAMMA",
                AP_GAMMA_ID,
            ),

            _ => {
                crate::log_smp_raw!("{}", "AP Scheduler: no test thread for CPU=",);
                crate::log_smp_raw!("{}", cpu_id as u64);
                crate::log_smp!("");

                loop {
                    unsafe {
                        core::arch::asm!("hlt");
                    }
                }
            }
        };

        /*
         * Add thread to this CPU's scheduler.
         */
        let added = unsafe {
            crate::task::SCHEDULERS[cpu_id]
                .add_thread(thread, cpu_id)
        };

        if !added {
            crate::log_test_raw!("ERROR: failed to add ");
            crate::log_smp!("{}", name);


            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        
        crate::log_smp_raw!("{}", name);
        crate::log_smp_raw!(" -> CPU ");
        crate::log_smp_raw!("{}", cpu_id as u64);
        crate::log_smp!("");

        crate::log_smp_raw!("{}", name);
        crate::log_smp_raw!(" -> PID 0, TID ");
        crate::log_smp_raw!("{}", thread_id as u64);
        crate::log_smp!("");

        unsafe {
            crate::task::SCHEDULERS[cpu_id].start_first();
        }
    }

    unsafe {
        core::arch::asm!(
            "sti",
            options(nomem, nostack, preserves_flags)
        );
    }

    let rflags: u64;

    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            out(reg) rflags,
            options(nomem, preserves_flags)
        );
    }

    crate::log_smp_raw!("AP: interrupts enabled, CPU=");
    crate::log_smp_raw!("{}", cpu_id as u64);

    crate::log_smp_raw!(" IF=");
    crate::log_smp_raw!("{}", (rflags >> 9) & 1);
    crate::log_smp!("");

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}