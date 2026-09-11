const TICK_FREQUENCY: u64 = 100;

pub fn init() {
    crate::drivers::serial::println("Clock initalized");
    crate::drivers::serial::print("Clock frequency: ");
    crate::drivers::serial::print_u64(TICK_FREQUENCY);
    crate::drivers::serial::println("hz");
}

#[inline]
pub fn ticks() -> u64 {
    crate::arch::x86_64::interrupts::ticks()
}

#[inline]
pub fn uptime_ms() -> u64 {
    ticks() * 1000 / TICK_FREQUENCY
}

#[inline]
pub fn uptime_seconds() -> u64 {
    uptime_ms() / 1000
}

pub fn test() {
    let mut start = uptime_ms();

    loop {
        let now = uptime_ms();

        if now >= start + 1000 {
            crate::drivers::serial::println("Clock test: 1000ms PASS");
            start = now;
        }

        core::hint::spin_loop();
    }
}