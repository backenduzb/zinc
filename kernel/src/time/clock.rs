pub const TICK_FREQUENCY: u64 =
    crate::drivers::timer::pit::PIT_BASE_FREQUENCY as u64
        / crate::drivers::timer::pit::PIT_DIVISOR as u64;

pub fn init() {
    crate::log_boot!("Clock initalized");
    crate::log_boot_raw!("Clock frequency: ");
    crate::log_boot_raw!("{}", TICK_FREQUENCY);
    crate::log_boot!("hz");
}

#[inline]
pub fn ticks() -> u64 {
    crate::arch::x86_64::interrupts::ticks()
}

#[inline]
pub fn uptime_ms() -> u64 {
    let divisor = crate::drivers::timer::pit::PIT_DIVISOR as u64;
    let base = crate::drivers::timer::pit::PIT_BASE_FREQUENCY as u64;
    loop {
        let before = ticks();
        let count = crate::drivers::timer::pit::counter();
        let after = ticks();
        if before == after {
            let elapsed = divisor.saturating_sub(count as u64).min(divisor);
            let total = before.saturating_mul(divisor).saturating_add(elapsed);
            return total.saturating_mul(1000) / base;
        }
    }
}

#[inline]
pub fn uptime_seconds() -> u64 {
    uptime_ms() / 1000
}

pub fn test() {
    crate::log_test!("CLOCK TEST: ENTER");

    let start = ticks();

    crate::log_vfs!("CLOCK TEST: TICKS READ");

    let mut spins: u64 = 0;
    const SPIN_TIMEOUT: u64 = 1_000_000_000;

    loop {
        let now = ticks();

        if now >= start + 100 {
            crate::log_sched_raw!("Clock test: 100 ticks PASS (ticks=");
            crate::log_boot_raw!("{}", now);
            crate::log_boot!(")");
            return;
        }

        spins += 1;

        if spins >= SPIN_TIMEOUT {
            crate::log_test!("Clock test: TIMEOUT, timer IRQ kelmayapti!");
            return;
        }

        core::hint::spin_loop();
    }
}