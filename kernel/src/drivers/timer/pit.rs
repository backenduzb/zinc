use crate::arch::x86_64::io::{io_wait, outb};

const PIT_COMMAND: u16 = 0x43;
const PIT_CHANNEL0: u16 = 0x40;

const PIT_FREQUENCY: u32 = 1_193_182;
const TIMER_REQUENCY: u32 = 100;

pub fn init() {
    let divisor = PIT_FREQUENCY / TIMER_REQUENCY;

    unsafe {
        outb(PIT_COMMAND, 0x036);
        io_wait();

        outb(PIT_CHANNEL0, (divisor & 0xFF) as u8);
        io_wait();
        outb(PIT_CHANNEL0, ((divisor >> 8) & 0xFF) as u8);
        io_wait();
    }

    crate::log_boot_raw!("PIT frequency: ");
    crate::log_boot_raw!("{}", TIMER_REQUENCY as u64);
    crate::log_boot!("Hz");
    
}