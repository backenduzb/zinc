use core::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}


fn diagnostic_ids() -> (i64, u64, u64) {
    let cr3 = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

    let gs: u64 = unsafe { crate::arch::x86_64::io::rdmsr(0xC0000101) };

    if gs == 0 {
        return (-1, 9999, cr3);
    }

    let cpu_id: usize = unsafe { *(gs as *const usize) };

    if cpu_id >= crate::arch::x86_64::smp::percpu::MAX_CPUS {
        return (-1, 9999, cr3);
    }

    let tid = unsafe {
        let base = core::ptr::addr_of_mut!(crate::task::SCHEDULERS);
        (&*base)[cpu_id].current_id().unwrap_or(9999)
    };

    (cpu_id as i64, tid, cr3)
}

fn fault_diagnostic(vector: u8, code: u64, rip: u64, cs: u64) {
    let (cpu, tid, cr3) = diagnostic_ids();

    let cr2: u64 = if vector == 14 {
        let address: u64;

        unsafe {
            core::arch::asm!(
                "mov {}, cr2",
                out(reg) address,
                options(nomem, nostack, preserves_flags)
            );
        }

        address
    } else {
        0
    };

    crate::drivers::serial::print("FAULT vector=");
    crate::drivers::serial::print_u64(vector as u64);
    crate::drivers::serial::print(" code=");
    crate::drivers::serial::print_u64(code);
    crate::drivers::serial::print(" RIP=0x");
    crate::drivers::serial::print_hex_u64(rip);
    crate::drivers::serial::print(" CS=0x");
    crate::drivers::serial::print_hex_u64(cs);

    if vector == 14 {
        crate::drivers::serial::print(" CR2=0x");
        crate::drivers::serial::print_hex_u64(cr2);
    }

    crate::drivers::serial::print(" CPU=");
    if cpu < 0 {
        crate::drivers::serial::print("?");
    } else {
        crate::drivers::serial::print_u64(cpu as u64);
    }
    crate::drivers::serial::print(" TID=");
    crate::drivers::serial::print_u64(tid);
    crate::drivers::serial::print(" CR3=0x");
    crate::drivers::serial::print_hex_u64(cr3);
    crate::drivers::serial::println("");
}
#[unsafe(no_mangle)]
pub extern "C" fn divide_error() -> ! {
    crate::drivers::serial::println("EXCEPTION: Driver Error (#DE)");

    crate::arch::x86_64::cpu::halt();
}

#[unsafe(no_mangle)]
pub extern "C" fn ud_fault_handler(rip: u64, cs: u64) -> ! {
    crate::drivers::serial::println("EXCEPTION: Invalid Opcode (#UD)");
    fault_diagnostic(6, 0, rip, cs);

    crate::arch::x86_64::cpu::halt();
}

#[unsafe(no_mangle)]
pub extern "C" fn gp_fault_handler(code: u64, rip: u64, cs: u64) -> ! {
    crate::drivers::serial::print("EXCEPTION: General Protection Fault (#GP), code=");
    crate::drivers::serial::print_u64(code);
    crate::drivers::serial::println("");
    fault_diagnostic(13, code, rip, cs);

    crate::arch::x86_64::cpu::halt();
}

#[unsafe(no_mangle)]
pub extern "C" fn page_fault_handler(code: u64, rip: u64, cs: u64) -> ! {
    let address: u64;

    unsafe {
        core::arch::asm!(
            "mov {}, cr2",
            out(reg) address,
            options(nomem, nostack, preserves_flags)
        );
    }

    crate::drivers::serial::print("EXCEPTION: Page Fault (#PF) at 0x");
    crate::drivers::serial::print_u64(address);
    crate::drivers::serial::print(", code=");
    crate::drivers::serial::print_u64(code);
    crate::drivers::serial::println("");
    fault_diagnostic(14, code, rip, cs);

    crate::arch::x86_64::cpu::halt();
}

#[unsafe(no_mangle)]
pub extern "C" fn double_fault_handler(code: u64, rip: u64, cs: u64) -> ! {
    crate::drivers::serial::print("EXCEPTION: Double Fault (#DF), code=");
    crate::drivers::serial::print_u64(code);
    crate::drivers::serial::println("");
    fault_diagnostic(8, code, rip, cs);

    crate::arch::x86_64::cpu::halt();
}

#[unsafe(no_mangle)]
pub extern "C" fn timer_handler(
    frame: *mut crate::arch::x86_64::context::InterruptFrame,
) -> (u64, u64) {
    let tick = TICKS.fetch_add(1, Ordering::Relaxed) + 1;

    if tick == 1 {
        crate::drivers::serial::println("TIMER IRQ: FIRST");
    }

    if tick == 100 {
        crate::drivers::serial::println("TIMER IRQ: 100");
    }

    crate::drivers::timer::pic::end_of_interrupt();

    (frame as u64, 0)
}

fn log_user_kernel_transition(frame: *mut crate::arch::x86_64::context::InterruptFrame) {
    if frame.is_null() {
        return;
    }

    let (cs, rip) = unsafe { ((*frame).cs, (*frame).rip) };

    if cs & 3 != 3 {
        return;
    }

    static TRANSITIONS: AtomicU64 = AtomicU64::new(0);
    let n = TRANSITIONS.fetch_add(1, Ordering::Relaxed);

    if n >= 2 {
        return;
    }

    let cr3 = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

    let gs: u64 = unsafe { crate::arch::x86_64::io::rdmsr(0xC0000101) };

    let (cpu_text, rsp0) = if gs == 0 {
        (9999u64, 0u64)
    } else {
        let rsp0 = unsafe { (*(gs as *const crate::arch::x86_64::smp::percpu::PerCpu)).tss.get().as_ref().map(|tss| tss.kernel_stack()).unwrap_or(0) };
        let cpu_id = unsafe { (*(gs as *const crate::arch::x86_64::smp::percpu::PerCpu)).cpu_id as u64 };
        (cpu_id, rsp0)
    };

    crate::drivers::serial::print("USER->KERNEL INTERRUPT: CPU=");
    crate::drivers::serial::print_u64(cpu_text);
    crate::drivers::serial::print(" RIP=0x");
    crate::drivers::serial::print_hex_u64(rip);
    crate::drivers::serial::print(" CS=0x");
    crate::drivers::serial::print_hex_u64(cs);
    crate::drivers::serial::print(" CR3=0x");
    crate::drivers::serial::print_hex_u64(cr3);
    crate::drivers::serial::print(" TSS.RSP0=0x");
    crate::drivers::serial::print_hex_u64(rsp0);
    crate::drivers::serial::println("");
    crate::drivers::serial::println("User -> Kernel interrupt succeeded");
}

#[unsafe(no_mangle)]
pub extern "C" fn lapic_timer_handler(
    frame: *mut crate::arch::x86_64::context::InterruptFrame,
) -> (u64, u64) {
    let cpu_id = crate::arch::x86_64::smp::percpu::current_cpu_id();

    log_user_kernel_transition(frame);

    if cpu_id != 0 {
        crate::arch::x86_64::lapic::ap_tick(cpu_id);

        crate::arch::x86_64::lapic::eoi();

        if frame.is_null() {
            return (frame as u64, 0);
        }

        return crate::task::scheduler_tick(frame);
    }

    let ticks = crate::arch::x86_64::lapic::tick();

    crate::arch::x86_64::lapic::eoi();

    if ticks <= 20 {
        crate::drivers::serial::print("LAPIC TIMER: ");
        crate::drivers::serial::print_u64(ticks);
        crate::drivers::serial::println("");
    }

    if frame.is_null() {
        crate::drivers::serial::println("LAPIC TIMER: BAD FRAME");
        return (frame as u64, 0);
    }

    crate::task::scheduler_tick(frame)
}
