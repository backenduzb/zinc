pub mod input;
pub mod thread;

pub fn init() {
    crate::log_boot!("TTY: initialized");
    crate::drivers::serial::print("zinc > ");
}