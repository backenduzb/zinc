pub mod input;
pub mod thread;
pub mod commands;

use crate::task::thread::ThreadID;

static mut KEYBOARD_TID: Option<ThreadID> = None;

pub fn init() {
    crate::log_boot!("TTY: initialized");
}

pub fn show_prompt() {
    crate::drivers::serial::print("zinc > ");
}

pub fn keyboard_tid() -> Option<ThreadID> {
    unsafe { KEYBOARD_TID }
}
