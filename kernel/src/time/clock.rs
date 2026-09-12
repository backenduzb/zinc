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
    let start = ticks();

    let mut spins: u64 = 0;
    const SPIN_TIMEOUT: u64 = 1_000_000_000;

    loop {
        let now = ticks();

        if now >= start + 100 {
            crate::drivers::serial::print("Clock test: 100 ticks PASS (ticks=");
            crate::drivers::serial::print_u64(now);
            crate::drivers::serial::println(")");
            return;
        }

        spins += 1;

        if spins >= SPIN_TIMEOUT {
            crate::drivers::serial::println("Clock test: TIMEOUT, timer IRQ kelmayapti!");
            return;
        }

        core::hint::spin_loop();
    }
}