use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86_64::{smp::percpu::MAX_CPUS};

const TRAMPOLINE_PHYS: u64 = 0x8000;

const CR3_OFFSET: usize = 0x100;
const STACK_OFFSET: usize = 0x108;
const ENTRY_OFFSET: usize = 0x110;

const TRAMPOLINE_SIZE: usize = 0x120;

static AP_STARTED: [AtomicBool; MAX_CPUS] = [const { AtomicBool::new(false) }; MAX_CPUS];

static TRAMPOLINE: &[u8] = include_bytes!("../../../../../boot/ap/build/trampoline.bin");

pub fn install(cr3: u64, stack: u64, entry: u64) {
    if TRAMPOLINE.len() < TRAMPOLINE_SIZE {
        crate::drivers::serial::println("SMP: trampoline too small");
        return;
    }

    let dest_phys = TRAMPOLINE_PHYS;

    let dest_virt = crate::boot::limine::physical_to_virtual(dest_phys)
        .expect("SMP: failed to translate trampoline physical address");

    crate::drivers::serial::print("SMP: trampoline physical: 0x");
    crate::drivers::serial::print_hex_u64(dest_phys);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("SMP: trampoline virtual: 0x");
    crate::drivers::serial::print_hex_u64(dest_virt);
    crate::drivers::serial::println("");

    unsafe {
        let dst = dest_virt as *mut u8;

        core::ptr::copy_nonoverlapping(TRAMPOLINE.as_ptr(), dst, TRAMPOLINE.len());

        core::ptr::write_unaligned(dst.add(CR3_OFFSET) as *mut u64, cr3);

        core::ptr::write_unaligned(dst.add(STACK_OFFSET) as *mut u64, stack);

        core::ptr::write_unaligned(dst.add(ENTRY_OFFSET) as *mut u64, entry);
    }

    crate::drivers::serial::println("SMP: AP trampoline installed");

    crate::drivers::serial::print("SMP: trampoline CR3: 0x");
    crate::drivers::serial::print_hex_u64(cr3);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("SMP: trampoline stack: 0x");
    crate::drivers::serial::print_hex_u64(stack);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("SMP: trampoline entry: 0x");
    crate::drivers::serial::print_hex_u64(entry);
    crate::drivers::serial::println("");
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

    crate::drivers::serial::print(
        "AP: entered trampoline, LAPIC ID = ",
    );
    crate::drivers::serial::print_u64(apic_id as u64);
    crate::drivers::serial::println("");

    if cpu_id < MAX_CPUS {
        AP_STARTED[cpu_id].store(true, Ordering::Release);

        crate::arch::x86_64::smp::percpu::init_ap(
            cpu_id,
            apic_id,
        );
    }

    let cpu = crate::arch::x86_64::smp::percpu::current();

    crate::drivers::serial::print("AP: PerCpu CPU ID = ");
    crate::drivers::serial::print_u64(cpu.cpu_id as u64);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("AP: PerCpu APIC ID = ");
    crate::drivers::serial::print_u64(cpu.apic_id as u64);
    crate::drivers::serial::println("");

    let cpu = crate::arch::x86_64::smp::percpu::current();
    
    crate::drivers::serial::print("AP: GS CPU ID = ");
    crate::drivers::serial::print_u64(cpu.cpu_id as u64);
    crate::drivers::serial::println("");
    
    crate::drivers::serial::print("AP: GS APIC ID = ");
    crate::drivers::serial::print_u64(cpu.apic_id as u64);
    crate::drivers::serial::println("");

    let cpu = crate::arch::x86_64::smp::percpu::current();
    
    crate::drivers::serial::print("SMP: GS current CPU = ");
    crate::drivers::serial::print_u64(cpu.cpu_id as u64);
    crate::drivers::serial::println("");

    crate::arch::x86_64::idt::load_ap();

    crate::arch::x86_64::lapic::init_ap();

    crate::arch::x86_64::lapic::timer_init_ap();
    crate::arch::x86_64::lapic::timer_start_ap(
        crate::arch::x86_64::lapic::TIMER_INITIAL_COUNT,
    );

    let cpu = crate::arch::x86_64::smp::percpu::current();
    cpu.set_timer_initialized();

    crate::drivers::serial::print("AP LAPIC: timer initialized, CPU=");
    crate::drivers::serial::print_u64(cpu.cpu_id as u64);
    crate::drivers::serial::println("");

    let cpu_id = crate::arch::x86_64::smp::percpu::current_cpu_id();

    if cpu_id != 0 && cpu_id < crate::arch::x86_64::smp::percpu::MAX_CPUS {
        use crate::task::test::{AP_BETA_ID, AP_GAMMA_ID, AP_SILF_ID};
        use crate::task::thread::Thread;

        unsafe {
            crate::task::SCHEDULERS[cpu_id].init_idle(cpu_id);
        }

        crate::drivers::serial::print("AP Scheduler initialized: CPU=");
        crate::drivers::serial::print_u64(cpu_id as u64);
        crate::drivers::serial::println("");

        let (thread, name) = match cpu_id {
            1 => (Thread::new(AP_BETA_ID, 1, crate::task::test::ap_beta), "AP_BETA"),
            2 => (Thread::new(AP_SILF_ID, 2, crate::task::test::ap_silf), "AP_SILF"),
            3 => (
                Thread::new(AP_GAMMA_ID, 3, crate::task::test::ap_gamma),
                "AP_GAMMA",
            ),
            _ => {
                crate::drivers::serial::print("AP Scheduler: no test thread for CPU=");
                crate::drivers::serial::print_u64(cpu_id as u64);
                crate::drivers::serial::println("");

                unsafe {
                    crate::task::SCHEDULERS[cpu_id].start_first();
                }
            }
        };

        unsafe {
            if !crate::task::SCHEDULERS[cpu_id].add_thread(thread, cpu_id) {
                crate::drivers::serial::print("ERROR: failed to add ");
                crate::drivers::serial::println(name);
            } else {
                crate::drivers::serial::print(name);
                crate::drivers::serial::print(" -> CPU ");
                crate::drivers::serial::print_u64(cpu_id as u64);
                crate::drivers::serial::println("");
            }
        }

        unsafe {
            crate::task::SCHEDULERS[cpu_id].start_first();
        }
    }

    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
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
    crate::drivers::serial::print("AP: interrupts enabled, CPU=");
    crate::drivers::serial::print_u64(cpu_id as u64);
    crate::drivers::serial::print(" IF=");
    crate::drivers::serial::print_u64((rflags >> 9) & 1);
    crate::drivers::serial::println("");

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}