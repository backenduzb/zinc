pub mod process;
pub mod thread;
pub mod scheduler;
pub mod test;

use scheduler::Scheduler;
use thread::ThreadState;

pub static mut SCHEDULER: Scheduler = Scheduler::new();

#[inline]
pub fn scheduler() -> *mut Scheduler {
    core::ptr::addr_of_mut!(SCHEDULER)
}

pub fn sleep(ticks: u64) {
    unsafe {
        (*scheduler()).sleep_current(ticks);
    }

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }

        let sleeping = unsafe {
            (*scheduler())
                .current()
                .map(|thread| thread.state() == ThreadState::Sleeping)
                .unwrap_or(false)
        };

        if !sleeping {
            break;
        }
    }
}

pub fn scheduler_tick(
    frame: *mut crate::arch::x86_64::context::InterruptFrame,
) -> (u64, u64) {
    unsafe {
        (*scheduler()).tick(frame)
    }
}