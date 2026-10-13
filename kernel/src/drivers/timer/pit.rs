use crate::arch::x86_64::io::{inb, io_wait, outb};

const PIT_COMMAND: u16 = 0x43;
const PIT_CHANNEL0: u16 = 0x40;

pub const PIT_BASE_FREQUENCY: u32 = 1_193_182;
pub const PIT_DIVISOR: u32 = PIT_BASE_FREQUENCY / 100;

pub fn init() {
    let divisor = PIT_DIVISOR;

    unsafe {
        outb(PIT_COMMAND, 0x036);
        io_wait();

        outb(PIT_CHANNEL0, (divisor & 0xFF) as u8);
        io_wait();
        outb(PIT_CHANNEL0, ((divisor >> 8) & 0xFF) as u8);
        io_wait();
    }

    crate::log_boot_raw!("PIT frequency: ");
    crate::log_boot_raw!("{}", (PIT_BASE_FREQUENCY / PIT_DIVISOR) as u64);
    crate::log_boot!("Hz");
}

#[inline]
pub fn counter() -> u16 {
    unsafe {
        outb(PIT_COMMAND, 0x00);
        let lo = inb(PIT_CHANNEL0) as u16;
        let hi = inb(PIT_CHANNEL0) as u16;
        (hi << 8) | lo
    }
}