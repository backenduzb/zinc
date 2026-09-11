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
pub extern "C" fn timer_handler() {
    let tick = TICKS.fetch_add(1, Ordering::Relaxed) + 1;

    if tick % 100 == 0 {
        crate::drivers::serial::print_u64(tick / 100);
        crate::drivers::serial::println("");
    }

    crate::drivers::timer::pic::end_of_interrupt();
}