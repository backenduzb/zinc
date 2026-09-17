use super::thread::Thread;

pub extern "C" fn thread_alpha() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::println("Thread ALPHA started!");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        crate::drivers::serial::print("ALPHA: ");
        crate::drivers::serial::print_u64(counter);
        crate::drivers::serial::println("");

        if counter == 5 {
            crate::drivers::serial::println("ALPHA exiting!");
            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}

pub extern "C" fn thread_beta() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::println("Thread BETA started!");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        crate::drivers::serial::print("BETA: ");
        crate::drivers::serial::print_u64(counter);
        crate::drivers::serial::println("");

        if counter == 2 {
            crate::drivers::serial::println("BETA waiting for ALPHA...");

            crate::task::join(1);

            crate::drivers::serial::println("BETA: ALPHA finished!");

            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}

pub extern "C" fn thread_silf() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::println("Thread SILF started!");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        crate::drivers::serial::print("SILF: ");
        crate::drivers::serial::print_u64(counter);
        crate::drivers::serial::println("");

        crate::task::sleep(5);

        if counter == 4 {
            crate::drivers::serial::println("SILF: killing BETA!");
            crate::task::kill(2);
        }

        if counter == 5 {
            crate::drivers::serial::println("EXIT TEST exiting!");
            crate::task::exit();
        }
    }
}

// pub fn scheduler_test() -> ! {
//     crate::drivers::serial::println("=== Context Switch Test ===");

//     let thread_a = Thread::new(1, thread_alpha);

//     let thread_b = Thread::new(2, thread_beta);

//     let thread_silf = Thread::new(3, thread_silf);

//     unsafe {
//         let scheduler = crate::task::scheduler();

//         (*scheduler).add_thread(thread_a);
//         (*scheduler).add_thread(thread_b);
//         (*scheduler).add_thread(thread_silf);

//         crate::drivers::serial::println("Thread A added");
//         crate::drivers::serial::println("Thread B added");
//         crate::drivers::serial::println("Thread SILF added");

//         crate::drivers::serial::println("Starting scheduler (preemptive, timer-driven)...");

//         (*scheduler).start_first();
//     }
// }

pub fn scheduler_test() -> ! {
    crate::drivers::serial::println("=== SCHEDULER TEST ===");

    let thread_a = Thread::new(1, 0, 0, thread_alpha);
    let thread_b = Thread::new(2, 0, 0, thread_beta);
    let thread_s = Thread::new(3, 0, 0, thread_silf);

    unsafe {
        let scheduler = crate::task::scheduler();

        if !(*scheduler).add_thread(thread_a, 0) {
            crate::drivers::serial::println("ERROR: failed to add ALPHA");
        }

        if !(*scheduler).add_thread(thread_b, 0) {
            crate::drivers::serial::println("ERROR: failed to add BETA");
        }

        if !(*scheduler).add_thread(thread_s, 0) {
            crate::drivers::serial::println("ERROR: failed to add SILF");
        }

        (*scheduler).init_idle(0);

        crate::drivers::serial::println("ALPHA -> CPU 0");
        crate::drivers::serial::println("BETA  -> CPU 0");
        crate::drivers::serial::println("SILF  -> CPU 0");
        crate::drivers::serial::println("IDLE  -> CPU 0");

        crate::drivers::serial::println("Starting scheduler...");

        (*scheduler).start_first();
    }
}

pub extern "C" fn thread_idle() -> ! {
    loop {
        unsafe {
            core::arch::asm!("sti", "hlt");
        }
    }
}

pub const AP_BETA_ID: super::thread::ThreadID = 10;
pub const AP_SILF_ID: super::thread::ThreadID = 11;
pub const AP_GAMMA_ID: super::thread::ThreadID = 12;

pub extern "C" fn ap_beta() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::print("AP BETA started CPU=");
    crate::drivers::serial::print_u64(crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
    crate::drivers::serial::println("");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        if counter % 10 == 0 {
            crate::drivers::serial::print("AP BETA CPU=");
            crate::drivers::serial::print_u64(
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64,
            );
            crate::drivers::serial::print(" counter=");
            crate::drivers::serial::print_u64(counter);
            crate::drivers::serial::println("");
        }

        if counter == 30 {
            crate::drivers::serial::print("AP BETA exiting CPU=");
            crate::drivers::serial::print_u64(
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64,
            );
            crate::drivers::serial::println("");
            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}

pub extern "C" fn ap_silf() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::print("AP SILF started CPU=");
    crate::drivers::serial::print_u64(crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
    crate::drivers::serial::println("");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        if counter % 10 == 0 {
            crate::drivers::serial::print("AP SILF CPU=");
            crate::drivers::serial::print_u64(
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64,
            );
            crate::drivers::serial::print(" counter=");
            crate::drivers::serial::print_u64(counter);
            crate::drivers::serial::println("");
        }

        if counter == 30 {
            crate::drivers::serial::print("AP SILF exiting CPU=");
            crate::drivers::serial::print_u64(
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64,
            );
            crate::drivers::serial::println("");
            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}

pub extern "C" fn ap_gamma() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::print("AP GAMMA started CPU=");
    crate::drivers::serial::print_u64(crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
    crate::drivers::serial::println("");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        if counter % 10 == 0 {
            crate::drivers::serial::print("AP GAMMA CPU=");
            crate::drivers::serial::print_u64(
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64,
            );
            crate::drivers::serial::print(" counter=");
            crate::drivers::serial::print_u64(counter);
            crate::drivers::serial::println("");
        }

        if counter == 30 {
            crate::drivers::serial::print("AP GAMMA exiting CPU=");
            crate::drivers::serial::print_u64(
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64,
            );
            crate::drivers::serial::println("");
            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}