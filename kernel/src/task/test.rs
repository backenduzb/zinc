use super::thread::Thread;

pub fn scheduler_test() -> ! {
    crate::drivers::serial::println("=== Context Switch Test ===");

    let thread_a = Thread::new(
        1,
        super::thread::thread_alpha,
    );

    let thread_b = Thread::new(
        2,
        super::thread::thread_beta,
    );

    let thread_silf = Thread::new(
        3,
        super::thread::thread_silf,
    );
    
    unsafe {
        let scheduler = crate::task::scheduler();

        (*scheduler).add_thread(thread_a);
        (*scheduler).add_thread(thread_b);
        (*scheduler).add_thread(thread_silf);

        crate::drivers::serial::println("Thread A added");
        crate::drivers::serial::println("Thread B added");
        crate::drivers::serial::println("Thread SILF added");

        crate::drivers::serial::println("Starting scheduler (preemptive, timer-driven)...");

        (*scheduler).start_first();
    }
}