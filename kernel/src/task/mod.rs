pub mod process;
pub mod thread;
pub mod scheduler;
pub mod test;

use scheduler::Scheduler;

pub static mut SCHEDULER: Scheduler = Scheduler::new();

#[inline]
pub fn scheduler() -> *mut Scheduler {
    core::ptr::addr_of_mut!(SCHEDULER)
}

// pub fn yield_now() {
//     unsafe {
//         (*scheduler()).yield_now();
//     }
// }

pub fn scheduler_tick(
    frame: *mut crate::arch::x86_64::context::InterruptFrame,
) -> (u64, u64) {
    unsafe {
        (*scheduler()).tick(frame)
    }
}