pub mod process;
pub mod thread;
pub mod scheduler;
pub mod test;

use scheduler::Scheduler;
use thread::ThreadState;

use crate::task::thread::ThreadID;
use crate::arch::x86_64::smp::percpu::MAX_CPUS;


pub static mut SCHEDULERS: [Scheduler; MAX_CPUS] = [
    const { Scheduler::new() };
    MAX_CPUS
];

#[inline]
pub fn scheduler() -> *mut Scheduler {
    let cpu_id = crate::arch::x86_64::smp::percpu::current_cpu_id();

    if cpu_id >= MAX_CPUS {
        panic!("SMP: invalid CPU ID");
    }

    unsafe {
        core::ptr::addr_of_mut!(SCHEDULERS[cpu_id])
    }
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

pub fn exit() -> ! {
    unsafe {
        (*scheduler()).exit_current();
    }

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub fn kill(id: ThreadID) -> bool {
    unsafe {
        (*scheduler()).kill(id)
    }
}

pub fn join(id: thread::ThreadID) {
    unsafe {
        (*scheduler()).join(id);
    }
}