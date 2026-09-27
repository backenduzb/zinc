pub mod process;
pub mod ring3;
pub mod scheduler;
pub mod test;
pub mod thread;

use process::ProcessManager;
use scheduler::Scheduler;
use thread::ThreadState;

use crate::arch::x86_64::smp::percpu::MAX_CPUS;
use crate::task::thread::ThreadID;

pub static mut SCHEDULERS: [Scheduler; MAX_CPUS] = [const { Scheduler::new() }; MAX_CPUS];

pub static mut PROCESS_MANAGER: ProcessManager = ProcessManager::new();

#[inline]
pub fn scheduler() -> *mut Scheduler {
    let cpu_id = crate::arch::x86_64::smp::percpu::current_cpu_id();

    if cpu_id >= MAX_CPUS {
        panic!("SMP: invalid CPU ID");
    }

    unsafe { core::ptr::addr_of_mut!(SCHEDULERS[cpu_id]) }
}

#[inline]
pub fn process_manager() -> *mut ProcessManager {
    unsafe { core::ptr::addr_of_mut!(PROCESS_MANAGER) }
}

pub fn sleep(ticks: u64) {
    unsafe {
        let sched = scheduler();

        if (*sched).count() == 0 || !(*sched).is_started() {
            return;
        }

        (*sched).sleep_current(ticks);
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

pub fn scheduler_tick(frame: *mut crate::arch::x86_64::context::InterruptFrame) -> (u64, u64) {
    unsafe { (*scheduler()).tick(frame) }
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
    unsafe { (*scheduler()).kill(id) }
}

pub fn join(id: thread::ThreadID) {
    unsafe {
        (*scheduler()).join(id);
    }
}

pub fn scheduler_for_cpu(cpu_id: usize) -> *mut Scheduler {
    if cpu_id >= MAX_CPUS {
        panic!("SMP: invalid CPU ID");
    }

    unsafe { core::ptr::addr_of_mut!(SCHEDULERS[cpu_id]) }
}

pub fn find_scheduler_with_space() -> Option<(usize, *mut Scheduler)> {
    unsafe {
        for cpu_id in 0..MAX_CPUS {
            let scheduler = core::ptr::addr_of_mut!(SCHEDULERS[cpu_id]);

            if (*scheduler).count() < 8 {
                return Some((cpu_id, scheduler));
            }
        }
    }

    None
}

pub fn wait() {
    unsafe {
        (*scheduler()).wait_current();
    }
}

pub fn wake_thread(tid: thread::ThreadID) -> bool {
    unsafe {
        for cpu_id in 0..MAX_CPUS {
            let scheduler = core::ptr::addr_of_mut!(SCHEDULERS[cpu_id]);

            if (*scheduler).wake_thread(tid) {
                return true;
            }
        }
    }

    false
}

pub fn wake_thread_and_reschedule(tid: thread::ThreadID) -> bool {
    unsafe {
        for cpu_id in 0..MAX_CPUS {
            let scheduler = core::ptr::addr_of_mut!(SCHEDULERS[cpu_id]);

            if (*scheduler).wake_thread(tid) {
                let current_cpu = crate::arch::x86_64::smp::percpu::current_cpu_id();

                if cpu_id != current_cpu {
                    let apic_id = cpu_id as u32;

                    crate::arch::x86_64::lapic::send_reschedule_ipi(apic_id);
                }

                return true;
            }
        }
    }

    false
}

pub fn scheduler_reschedule(
    frame: *mut crate::arch::x86_64::context::InterruptFrame,
) -> (u64, u64) {
    unsafe { (*scheduler()).reschedule(frame) }
}
