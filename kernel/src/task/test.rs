use super::scheduler::Scheduler;
use super::thread::Thread;

pub fn scheduler_test() {
    crate::drivers::serial::println("=== Scheduler Test ===");

    let thread_a = Thread::new(
        1,
        super::thread::thread_alpha,
    );

    let thread_b = Thread::new(
        2,
        super::thread::thread_beta,
    );

    let mut scheduler = Scheduler::new();

    if scheduler.add_thread(thread_a) {
        crate::drivers::serial::println("Thread A added");
    } else {
        crate::drivers::serial::println("ERROR: failed to add Thread A");
    }

    if scheduler.add_thread(thread_b) {
        crate::drivers::serial::println("Thread B added");
    } else {
        crate::drivers::serial::println("ERROR: failed to add Thread B");
    }

    crate::drivers::serial::print("Thread count: ");
    crate::drivers::serial::print_u64(scheduler.count() as u64);
    crate::drivers::serial::println("");

    if let Some(id) = scheduler.current_id() {
        crate::drivers::serial::print("Current thread: ");
        crate::drivers::serial::print_u64(id);
        crate::drivers::serial::println("");
    }

    if let Some(index) = scheduler.next() {
        crate::drivers::serial::print("Next thread index: ");
        crate::drivers::serial::print_u64(index as u64);
        crate::drivers::serial::println("");
    }

    if let Some(id) = scheduler.current_id() {
        crate::drivers::serial::print("Current thread after next: ");
        crate::drivers::serial::print_u64(id);
        crate::drivers::serial::println("");
    }

    crate::drivers::serial::println("Scheduler test: OK");
}