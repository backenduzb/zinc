use crate::task::thread::Thread;

pub extern "C" fn keyboard_thread() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::log_boot!("TTY keyboard thread started");

    loop {
        super::input::poll();
        // crate::task::sleep(1);
    }
}

pub fn spawn() -> bool {
    unsafe {
        let process_manager = crate::task::process_manager();

        let Some((cpu_id, scheduler)) =
            crate::task::find_scheduler_with_space()
        else {
            crate::log_boot!("TTY: all CPU schedulers are full");
            return false;
        };

        let tid = (*process_manager).allocate_thread_id();

        let thread = Thread::new(tid, cpu_id, 0, keyboard_thread);

        crate::log_boot_raw!("TTY: CPU=");
        crate::log_boot_raw!("{}", cpu_id);
        crate::log_boot_raw!(" scheduler_count=");
        crate::log_boot_raw!("{}", (*scheduler).count());
        crate::log_boot!("");

        if !(*scheduler).add_thread(thread, cpu_id) {
            crate::log_boot!("TTY: failed to add keyboard thread");
            return false;
        }

        if !(*process_manager).add_thread(0, tid) {
            crate::log_boot!("TTY: failed to register keyboard thread");
            return false;
        }

        crate::log_boot_raw!("TTY keyboard thread TID=");
        crate::log_boot_raw!("{}", tid);
        crate::log_boot!("");

        true
    }
}