use super::thread::Thread;
use core::sync::atomic::{AtomicU64, Ordering};

pub static ALPHA_TID: AtomicU64 = AtomicU64::new(1);
pub static BETA_TID: AtomicU64 = AtomicU64::new(2);

pub extern "C" fn thread_alpha() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::log_vfs!("Thread ALPHA started!");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        crate::log_test_raw!("ALPHA: ");
        crate::log_test_raw!("{}", counter);
        crate::log_test!("");

        if counter == 5 {
            crate::log_test!("ALPHA exiting!");
            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}

pub extern "C" fn thread_beta() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::log_vfs!("Thread BETA started!");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        crate::log_test_raw!("BETA: ");
        crate::log_test_raw!("{}", counter);
        crate::log_test!("");

        if counter == 2 {
            crate::log_test!("BETA waiting for ALPHA...");

            let tid = ALPHA_TID.load(Ordering::Relaxed);
            crate::task::join(tid);

            crate::log_test!("BETA: ALPHA finished!");

            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}

pub extern "C" fn thread_silf() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::log_vfs!("Thread SILF started!");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        crate::log_test_raw!("SILF: ");
        crate::log_test_raw!("{}", counter);
        crate::log_test!("");

        crate::task::sleep(5);

        if counter == 4 {
            crate::log_test!("SILF: killing BETA!");
            let tid = BETA_TID.load(Ordering::Relaxed);
            crate::task::kill(tid);
        }

        if counter == 5 {
            crate::log_test!("EXIT TEST exiting!");
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
    crate::log_sched!("=== SCHEDULER TEST ===");

    unsafe {
        let process_manager = crate::task::process_manager();
        let tid_a = (*process_manager).allocate_thread_id();
        let tid_b = (*process_manager).allocate_thread_id();
        let tid_s = (*process_manager).allocate_thread_id();

        ALPHA_TID.store(tid_a, Ordering::Relaxed);
        BETA_TID.store(tid_b, Ordering::Relaxed);

        let thread_a = Thread::new(tid_a, 0, 0, thread_alpha);
        let thread_b = Thread::new(tid_b, 0, 0, thread_beta);
        let thread_s = Thread::new(tid_s, 0, 0, thread_silf);

        let scheduler = crate::task::scheduler();

        if !(*scheduler).add_thread(thread_a, 0) {
            crate::log_test!("ERROR: failed to add ALPHA");
        } else {
            (*process_manager).add_thread(0, tid_a);
        }

        if !(*scheduler).add_thread(thread_b, 0) {
            crate::log_test!("ERROR: failed to add BETA");
        } else {
            (*process_manager).add_thread(0, tid_b);
        }

        if !(*scheduler).add_thread(thread_s, 0) {
            crate::log_test!("ERROR: failed to add SILF");
        } else {
            (*process_manager).add_thread(0, tid_s);
        }

        (*scheduler).init_idle(0);

        crate::log_smp!("ALPHA -> CPU 0");
        crate::log_smp!("BETA  -> CPU 0");
        crate::log_smp!("SILF  -> CPU 0");
        crate::log_sched!("IDLE  -> CPU 0");

        if let Some(process) = (*process_manager).get(0) {
            crate::log_vfs_raw!("Kernel process threads: ");
            crate::log_test_raw!("{}", process.thread_count() as u64);
            crate::log_test!("");
        }

        crate::log_sched!("Starting scheduler...");

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

    crate::log_smp_raw!("AP BETA started CPU=");
    crate::log_test_raw!("{}", crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
    crate::log_test!("");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        if counter % 10 == 0 {
            crate::log_smp_raw!("AP BETA CPU=");
            crate::log_test_raw!("{}", crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
            crate::log_test_raw!(" counter=");
            crate::log_test_raw!("{}", counter);
            crate::log_test!("");
        }

        if counter == 30 {
            crate::log_smp_raw!("AP BETA exiting CPU=");
            crate::log_test_raw!("{}", crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
            crate::log_test!("");
            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}

pub extern "C" fn ap_silf() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::log_smp_raw!("AP SILF started CPU=");
    crate::log_test_raw!("{}", crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
    crate::log_test!("");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        if counter % 10 == 0 {
            crate::log_smp_raw!("AP SILF CPU=");
            crate::log_test_raw!("{}", crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
            crate::log_test_raw!(" counter=");
            crate::log_test_raw!("{}", counter);
            crate::log_test!("");
        }

        if counter == 30 {
            crate::log_smp_raw!("AP SILF exiting CPU=");
            crate::log_test_raw!("{}", crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
            crate::log_test!("");
            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}

pub extern "C" fn ap_gamma() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::log_smp_raw!("AP GAMMA started CPU=");
    crate::log_test_raw!("{}", crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
    crate::log_test!("");

    let mut counter: u64 = 0;

    loop {
        counter += 1;

        if counter % 10 == 0 {
            crate::log_smp_raw!("AP GAMMA CPU=");
            crate::log_test_raw!("{}", crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
            crate::log_test_raw!(" counter=");
            crate::log_test_raw!("{}", counter);
            crate::log_test!("");
        }

        if counter == 30 {
            crate::log_smp_raw!("AP GAMMA exiting CPU=");
            crate::log_test_raw!("{}", crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
            crate::log_test!("");
            crate::task::exit();
        }

        crate::task::sleep(5);
    }
}

pub fn process_address_space_test() {
    use super::process::{Process, ProcessManager};
    use crate::arch::x86_64::paging::PageTable;
    use crate::boot::limine::physical_to_virtual;
    use crate::memory::physical::FrameAllocator;

    crate::log_test!("=== PROCESS ADDRESS SPACE TEST ===");

    let kernel = Process::new(0);
    if !kernel.is_kernel() || kernel.has_user_address_space() {
        crate::log_test!("PROCESS TEST: kernel distinction FAILED");
        return;
    }
    if kernel.address_space().is_some() || kernel.pml4_phys().is_some() {
        crate::log_test!("PROCESS TEST: kernel accessors FAILED");
        return;
    }
    crate::log_test!("Kernel process (no AddressSpace): OK");

    let mut allocator = FrameAllocator::new();
    let mut manager = ProcessManager::new();

    let pid = match manager.create_user(&mut allocator, None) {
        Some(pid) => pid,
        None => {
            crate::log_ring3!("PROCESS TEST: create_user FAILED");
            return;
        }
    };

    let process = match manager.get(pid) {
        Some(process) => process,
        None => {
            crate::log_test!("PROCESS TEST: lookup FAILED");
            return;
        }
    };

    if !process.is_user() || !process.has_user_address_space() {
        crate::log_ring3!("PROCESS TEST: user flag FAILED");
        return;
    }

    let pml4_phys = match process.pml4_phys() {
        Some(addr) if addr != 0 && addr % 4096 == 0 => addr,
        _ => {
            crate::log_test!("PROCESS TEST: invalid PML4");
            return;
        }
    };

    crate::log_ring3_raw!("User process PML4: 0x");
    crate::log_test_raw!("{}", pml4_phys);
    crate::log_test!("");

    let pml4_va = match physical_to_virtual(pml4_phys) {
        Some(va) => va,
        None => {
            crate::log_boot!("PROCESS TEST: HHDM unavailable");
            return;
        }
    };

    let present_upper = unsafe {
        let table = &*(pml4_va as *const PageTable);
        (256..512)
            .filter(|&i| table.entries[i].is_present())
            .count()
    };

    if present_upper == 0 {
        crate::log_test!("PROCESS TEST: kernel mappings MISSING");
        return;
    }

    crate::log_test_raw!("Kernel-half present entries: ");
    crate::log_test_raw!("{}", present_upper as u64);
    crate::log_test!("");

    if process.address_space().is_none() || manager.count() != 1 {
        crate::log_test!("PROCESS TEST: accessor/count FAILED");
        return;
    }

    crate::log_test!("PROCESS ADDRESS SPACE TEST: PASS");
}

pub fn user_thread_test() {
    use super::thread::{Thread, ThreadKind};

    crate::log_vfs!("=== USER THREAD TEST ===");

    let process_id = 1;
    let user_rip = 0x400000u64;
    let user_rsp = 0x0000_7FFF_FFFF_FFF0u64;

    let thread = Thread::new_user(100, 0, process_id, user_rip, user_rsp);

    if thread.kind() != ThreadKind::User || !thread.is_user() || thread.is_kernel() {
        crate::log_vfs!("USER THREAD TEST: kind FAILED");
        return;
    }
    crate::log_ring3!("Kind: User OK");

    if thread.process_id() != process_id {
        crate::log_vfs!("USER THREAD TEST: process_id FAILED");
        return;
    }
    crate::log_test!("Process ID: OK");

    if thread.user_rip() != user_rip || thread.user_rsp() != user_rsp {
        crate::log_vfs!("USER THREAD TEST: user rip/rsp FAILED");
        return;
    }
    crate::log_ring3!("User RIP/RSP stored: OK");

    if thread.stack_top() == 0 {
        crate::log_vfs!("USER THREAD TEST: kernel stack MISSING");
        return;
    }
    crate::log_test!("Kernel stack exists: OK");

    let frame_rsp = thread.interrupt_rsp();
    if frame_rsp == 0 || frame_rsp % 16 != 0 {
        crate::log_vfs!("USER THREAD TEST: frame alignment FAILED");
        return;
    }
    if thread.user_rsp() == frame_rsp {
        crate::log_vfs!("USER THREAD TEST: stack aliasing FAILED");
        return;
    }
    crate::log_test!("Frame alignment: OK");

    let frame = unsafe { &*thread.user_frame_ptr() };

    if frame.cs != 0x2B {
        crate::log_vfs!("USER THREAD TEST: CS FAILED");
        return;
    }
    if frame.ss != 0x33 {
        crate::log_vfs!("USER THREAD TEST: SS FAILED");
        return;
    }
    crate::log_test!("CS=0x2B SS=0x33: OK");

    if frame.rflags & 0x2 == 0 || frame.rflags & (1 << 9) == 0 {
        crate::log_vfs!("USER THREAD TEST: RFLAGS FAILED");
        return;
    }
    crate::log_test_raw!("RFLAGS=0x");
    crate::log_test_raw!("{}", frame.rflags);
    crate::log_test!(" IF set: OK");

    if frame.rip != user_rip || frame.rsp != user_rsp {
        crate::log_vfs!("USER THREAD TEST: frame rip/rsp FAILED");
        return;
    }
    crate::log_test!("Frame RIP/RSP: OK");

    if frame.rax != 0
        || frame.rbx != 0
        || frame.rcx != 0
        || frame.rdx != 0
        || frame.rsi != 0
        || frame.rdi != 0
        || frame.rbp != 0
        || frame.r8 != 0
        || frame.r9 != 0
        || frame.r10 != 0
        || frame.r11 != 0
        || frame.r12 != 0
        || frame.r13 != 0
        || frame.r14 != 0
        || frame.r15 != 0
    {
        crate::log_vfs!("USER THREAD TEST: GPR zero FAILED");
        return;
    }
    crate::log_test!("GPRs zero: OK");

    let kernel_thread = Thread::new(101, 0, 0, thread_idle);
    if kernel_thread.kind() != ThreadKind::Kernel || !kernel_thread.is_kernel() {
        crate::log_vfs!("USER THREAD TEST: kernel kind FAILED");
        return;
    }
    crate::log_vfs!("Kernel thread path unchanged: OK");

    crate::log_vfs!("USER THREAD TEST: PASS");
}

pub fn user_process_cr3_test() {
    use super::process::ProcessManager;
    use super::thread::Thread;

    crate::log_cr3!("=== USER PROCESS CR3 TEST ===");

    let kernel_cr3 = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

    crate::log_cr3_raw!("Kernel CR3: 0x");
    crate::log_test_raw!("{}", kernel_cr3);
    crate::log_test!("");

    if kernel_cr3 == 0 || kernel_cr3 % 4096 != 0 {
        crate::log_cr3!("CR3 TEST: bad kernel CR3");
        return;
    }

    let mut allocator = crate::memory::physical::FrameAllocator::new();
    let mut manager = ProcessManager::new();

    let pid = match manager.create_user(&mut allocator, None) {
        Some(pid) => pid,
        None => {
            crate::log_cr3!("CR3 TEST: create_user FAILED");
            return;
        }
    };

    let thread = Thread::new_user(200, 0, pid, 0x400000, 0x0000_7FFF_FFFF_FFF0);

    if thread.process_id() != pid {
        crate::log_cr3!("CR3 TEST: thread process link FAILED");
        return;
    }
    crate::log_vfs!("Thread -> Process link: OK");

    let pml4_phys = match manager.get(thread.process_id()).and_then(|p| p.pml4_phys()) {
        Some(addr) => addr,
        None => {
            crate::log_cr3!("CR3 TEST: pml4_phys MISSING");
            return;
        }
    };

    if pml4_phys == 0 {
        crate::log_cr3!("CR3 TEST: pml4 zero FAILED");
        return;
    }
    crate::log_test!("PML4 non-zero: OK");

    if pml4_phys % 4096 != 0 {
        crate::log_cr3!("CR3 TEST: pml4 alignment FAILED");
        return;
    }
    crate::log_boot!("PML4 page aligned: OK");

    if pml4_phys == kernel_cr3 {
        crate::log_cr3!("CR3 TEST: user PML4 == kernel CR3 FAILED");
        return;
    }

    crate::log_ring3_raw!("User PML4: 0x");
    crate::log_test_raw!("{}", pml4_phys);
    crate::log_cr3!(" (differs from kernel CR3: OK)");

    let user_va = match crate::boot::limine::physical_to_virtual(pml4_phys) {
        Some(va) => va,
        None => {
            crate::log_cr3!("CR3 TEST: HHDM unavailable");
            return;
        }
    };

    let kernel_va = match crate::boot::limine::physical_to_virtual(kernel_cr3) {
        Some(va) => va,
        None => {
            crate::log_cr3!("CR3 TEST: kernel PML4 unmapped");
            return;
        }
    };

    let mut present = 0usize;

    unsafe {
        let user_table = &*(user_va as *const crate::arch::x86_64::paging::PageTable);
        let kernel_table = &*(kernel_va as *const crate::arch::x86_64::paging::PageTable);

        for index in 256..512 {
            if user_table.entries[index].raw() != kernel_table.entries[index].raw() {
                crate::log_cr3_raw!("CR3 TEST: kernel-half mismatch at ");
                crate::log_test_raw!("{}", index as u64);
                crate::log_test!("");
                return;
            }

            if user_table.entries[index].is_present() {
                present += 1;
            }
        }
    }

    if present == 0 {
        crate::log_cr3!("CR3 TEST: kernel mappings MISSING");
        return;
    }

    crate::log_test_raw!("Kernel-half identical, present entries: ");
    crate::log_test_raw!("{}", present as u64);
    crate::log_test!("");

    let canary: u64 = 0xC3A0_1234_5678_9ABC;

    unsafe {
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));

        crate::arch::x86_64::paging::load_cr3(pml4_phys);

        let loaded = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

        if loaded != pml4_phys {
            crate::arch::x86_64::paging::load_cr3(kernel_cr3);
            core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
            crate::log_cr3!("CR3 TEST: load/readback FAILED");
            return;
        }

        let mut probe: u64 = 0;
        core::ptr::write_volatile(&mut probe, canary);
        let check = core::ptr::read_volatile(&probe);

        if check != canary {
            crate::arch::x86_64::paging::load_cr3(kernel_cr3);
            core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
            crate::log_cr3!("CR3 TEST: stack probe FAILED");
            return;
        }

        crate::arch::x86_64::paging::load_cr3(kernel_cr3);

        let restored = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));

        if restored != kernel_cr3 {
            crate::log_cr3!("CR3 TEST: restore FAILED");
            return;
        }
    }

    crate::log_cr3!("CR3 load/readback/restore: OK");
    crate::log_cr3!("USER PROCESS CR3 TEST: PASS");
}
