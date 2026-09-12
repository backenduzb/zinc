use core::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}
#[unsafe(no_mangle)]
pub extern "C" fn divide_error() -> ! {
    crate::drivers::serial::println("EXCEPTION: Driver Error (#DE)");

    crate::arch::x86_64::cpu::halt();
}

#[unsafe(no_mangle)]
pub extern "C" fn gp_fault_handler(code: u64) -> ! {
    crate::drivers::serial::print(
        "EXCEPTION: General Protection Fault (#GP), code=",
    );
    crate::drivers::serial::print_u64(code);
    crate::drivers::serial::println("");

    crate::arch::x86_64::cpu::halt();
}

#[unsafe(no_mangle)]
pub extern "C" fn page_fault_handler(code: u64) -> ! {
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

    crate::arch::x86_64::cpu::halt();
}

#[unsafe(no_mangle)]
pub extern "C" fn double_fault_handler(code: u64) -> ! {
    crate::drivers::serial::print(
        "EXCEPTION: Double Fault (#DF), code=",
    );
    crate::drivers::serial::print_u64(code);
    crate::drivers::serial::println("");

    crate::arch::x86_64::cpu::halt();
}

#[unsafe(no_mangle)]
pub extern "C" fn timer_handler(
    frame: *mut crate::arch::x86_64::context::InterruptFrame,
) -> (u64, u64) {
    TICKS.fetch_add(1, Ordering::Relaxed);

    let next = crate::task::scheduler_tick(frame);

    crate::drivers::timer::pic::end_of_interrupt();

    next
}