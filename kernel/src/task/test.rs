use super::thread::Thread;
use core::sync::atomic::{AtomicU64, Ordering};

pub static ALPHA_TID: AtomicU64 = AtomicU64::new(1);
pub static BETA_TID: AtomicU64 = AtomicU64::new(2);

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

            let tid = ALPHA_TID.load(Ordering::Relaxed);
            crate::task::join(tid);

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
            let tid = BETA_TID.load(Ordering::Relaxed);
            crate::task::kill(tid);
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
            crate::drivers::serial::println("ERROR: failed to add ALPHA");
        } else {
            (*process_manager).add_thread(0, tid_a);
        }

        if !(*scheduler).add_thread(thread_b, 0) {
            crate::drivers::serial::println("ERROR: failed to add BETA");
        } else {
            (*process_manager).add_thread(0, tid_b);
        }

        if !(*scheduler).add_thread(thread_s, 0) {
            crate::drivers::serial::println("ERROR: failed to add SILF");
        } else {
            (*process_manager).add_thread(0, tid_s);
        }

        (*scheduler).init_idle(0);

        crate::drivers::serial::println("ALPHA -> CPU 0");
        crate::drivers::serial::println("BETA  -> CPU 0");
        crate::drivers::serial::println("SILF  -> CPU 0");
        crate::drivers::serial::println("IDLE  -> CPU 0");

        if let Some(process) = (*process_manager).get(0) {
            crate::drivers::serial::print("Kernel process threads: ");
            crate::drivers::serial::print_u64(process.thread_count() as u64);
            crate::drivers::serial::println("");
        }

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
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64
            );
            crate::drivers::serial::print(" counter=");
            crate::drivers::serial::print_u64(counter);
            crate::drivers::serial::println("");
        }

        if counter == 30 {
            crate::drivers::serial::print("AP BETA exiting CPU=");
            crate::drivers::serial::print_u64(
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64
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
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64
            );
            crate::drivers::serial::print(" counter=");
            crate::drivers::serial::print_u64(counter);
            crate::drivers::serial::println("");
        }

        if counter == 30 {
            crate::drivers::serial::print("AP SILF exiting CPU=");
            crate::drivers::serial::print_u64(
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64
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
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64
            );
            crate::drivers::serial::print(" counter=");
            crate::drivers::serial::print_u64(counter);
            crate::drivers::serial::println("");
        }

        if counter == 30 {
            crate::drivers::serial::print("AP GAMMA exiting CPU=");
            crate::drivers::serial::print_u64(
                crate::arch::x86_64::smp::percpu::current_cpu_id() as u64
            );
            crate::drivers::serial::println("");
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

    crate::drivers::serial::println("=== PROCESS ADDRESS SPACE TEST ===");

    let kernel = Process::new(0);
    if !kernel.is_kernel() || kernel.has_user_address_space() {
        crate::drivers::serial::println("PROCESS TEST: kernel distinction FAILED");
        return;
    }
    if kernel.address_space().is_some() || kernel.pml4_phys().is_some() {
        crate::drivers::serial::println("PROCESS TEST: kernel accessors FAILED");
        return;
    }
    crate::drivers::serial::println("Kernel process (no AddressSpace): OK");

    let mut allocator = FrameAllocator::new();
    let mut manager = ProcessManager::new();

    let pid = match manager.create_user(&mut allocator, None) {
        Some(pid) => pid,
        None => {
            crate::drivers::serial::println("PROCESS TEST: create_user FAILED");
            return;
        }
    };

    let process = match manager.get(pid) {
        Some(process) => process,
        None => {
            crate::drivers::serial::println("PROCESS TEST: lookup FAILED");
            return;
        }
    };

    if !process.is_user() || !process.has_user_address_space() {
        crate::drivers::serial::println("PROCESS TEST: user flag FAILED");
        return;
    }

    let pml4_phys = match process.pml4_phys() {
        Some(addr) if addr != 0 && addr % 4096 == 0 => addr,
        _ => {
            crate::drivers::serial::println("PROCESS TEST: invalid PML4");
            return;
        }
    };

    crate::drivers::serial::print("User process PML4: 0x");
    crate::drivers::serial::print_u64(pml4_phys);
    crate::drivers::serial::println("");

    let pml4_va = match physical_to_virtual(pml4_phys) {
        Some(va) => va,
        None => {
            crate::drivers::serial::println("PROCESS TEST: HHDM unavailable");
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
        crate::drivers::serial::println("PROCESS TEST: kernel mappings MISSING");
        return;
    }

    crate::drivers::serial::print("Kernel-half present entries: ");
    crate::drivers::serial::print_u64(present_upper as u64);
    crate::drivers::serial::println("");

    if process.address_space().is_none() || manager.count() != 1 {
        crate::drivers::serial::println("PROCESS TEST: accessor/count FAILED");
        return;
    }

    crate::drivers::serial::println("PROCESS ADDRESS SPACE TEST: PASS");
}

pub fn user_thread_test() {
    use super::thread::{Thread, ThreadKind};

    crate::drivers::serial::println("=== USER THREAD TEST ===");

    let process_id = 1;
    let user_rip = 0x400000u64;
    let user_rsp = 0x0000_7FFF_FFFF_FFF0u64;

    let thread = Thread::new_user(100, 0, process_id, user_rip, user_rsp);

    if thread.kind() != ThreadKind::User || !thread.is_user() || thread.is_kernel() {
        crate::drivers::serial::println("USER THREAD TEST: kind FAILED");
        return;
    }
    crate::drivers::serial::println("Kind: User OK");

    if thread.process_id() != process_id {
        crate::drivers::serial::println("USER THREAD TEST: process_id FAILED");
        return;
    }
    crate::drivers::serial::println("Process ID: OK");

    if thread.user_rip() != user_rip || thread.user_rsp() != user_rsp {
        crate::drivers::serial::println("USER THREAD TEST: user rip/rsp FAILED");
        return;
    }
    crate::drivers::serial::println("User RIP/RSP stored: OK");

    if thread.stack_top() == 0 {
        crate::drivers::serial::println("USER THREAD TEST: kernel stack MISSING");
        return;
    }
    crate::drivers::serial::println("Kernel stack exists: OK");

    let frame_rsp = thread.interrupt_rsp();
    if frame_rsp == 0 || frame_rsp % 16 != 0 {
        crate::drivers::serial::println("USER THREAD TEST: frame alignment FAILED");
        return;
    }
    if thread.user_rsp() == frame_rsp {
        crate::drivers::serial::println("USER THREAD TEST: stack aliasing FAILED");
        return;
    }
    crate::drivers::serial::println("Frame alignment: OK");

    let frame = unsafe { &*thread.user_frame_ptr() };

    if frame.cs != 0x2B {
        crate::drivers::serial::println("USER THREAD TEST: CS FAILED");
        return;
    }
    if frame.ss != 0x33 {
        crate::drivers::serial::println("USER THREAD TEST: SS FAILED");
        return;
    }
    crate::drivers::serial::println("CS=0x2B SS=0x33: OK");

    if frame.rflags & 0x2 == 0 || frame.rflags & (1 << 9) == 0 {
        crate::drivers::serial::println("USER THREAD TEST: RFLAGS FAILED");
        return;
    }
    crate::drivers::serial::print("RFLAGS=0x");
    crate::drivers::serial::print_u64(frame.rflags);
    crate::drivers::serial::println(" IF set: OK");

    if frame.rip != user_rip || frame.rsp != user_rsp {
        crate::drivers::serial::println("USER THREAD TEST: frame rip/rsp FAILED");
        return;
    }
    crate::drivers::serial::println("Frame RIP/RSP: OK");

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
        crate::drivers::serial::println("USER THREAD TEST: GPR zero FAILED");
        return;
    }
    crate::drivers::serial::println("GPRs zero: OK");

    let kernel_thread = Thread::new(101, 0, 0, thread_idle);
    if kernel_thread.kind() != ThreadKind::Kernel || !kernel_thread.is_kernel() {
        crate::drivers::serial::println("USER THREAD TEST: kernel kind FAILED");
        return;
    }
    crate::drivers::serial::println("Kernel thread path unchanged: OK");

    crate::drivers::serial::println("USER THREAD TEST: PASS");
}

pub fn user_process_cr3_test() {
    use super::process::ProcessManager;
    use super::thread::Thread;

    crate::drivers::serial::println("=== USER PROCESS CR3 TEST ===");

    let kernel_cr3 = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

    crate::drivers::serial::print("Kernel CR3: 0x");
    crate::drivers::serial::print_u64(kernel_cr3);
    crate::drivers::serial::println("");

    if kernel_cr3 == 0 || kernel_cr3 % 4096 != 0 {
        crate::drivers::serial::println("CR3 TEST: bad kernel CR3");
        return;
    }

    let mut allocator = crate::memory::physical::FrameAllocator::new();
    let mut manager = ProcessManager::new();

    let pid = match manager.create_user(&mut allocator, None) {
        Some(pid) => pid,
        None => {
            crate::drivers::serial::println("CR3 TEST: create_user FAILED");
            return;
        }
    };

    let thread = Thread::new_user(200, 0, pid, 0x400000, 0x0000_7FFF_FFFF_FFF0);

    if thread.process_id() != pid {
        crate::drivers::serial::println("CR3 TEST: thread process link FAILED");
        return;
    }
    crate::drivers::serial::println("Thread -> Process link: OK");

    let pml4_phys = match manager.get(thread.process_id()).and_then(|p| p.pml4_phys()) {
        Some(addr) => addr,
        None => {
            crate::drivers::serial::println("CR3 TEST: pml4_phys MISSING");
            return;
        }
    };

    if pml4_phys == 0 {
        crate::drivers::serial::println("CR3 TEST: pml4 zero FAILED");
        return;
    }
    crate::drivers::serial::println("PML4 non-zero: OK");

    if pml4_phys % 4096 != 0 {
        crate::drivers::serial::println("CR3 TEST: pml4 alignment FAILED");
        return;
    }
    crate::drivers::serial::println("PML4 page aligned: OK");

    if pml4_phys == kernel_cr3 {
        crate::drivers::serial::println("CR3 TEST: user PML4 == kernel CR3 FAILED");
        return;
    }

    crate::drivers::serial::print("User PML4: 0x");
    crate::drivers::serial::print_u64(pml4_phys);
    crate::drivers::serial::println(" (differs from kernel CR3: OK)");

    let user_va = match crate::boot::limine::physical_to_virtual(pml4_phys) {
        Some(va) => va,
        None => {
            crate::drivers::serial::println("CR3 TEST: HHDM unavailable");
            return;
        }
    };

    let kernel_va = match crate::boot::limine::physical_to_virtual(kernel_cr3) {
        Some(va) => va,
        None => {
            crate::drivers::serial::println("CR3 TEST: kernel PML4 unmapped");
            return;
        }
    };

    let mut present = 0usize;

    unsafe {
        let user_table = &*(user_va as *const crate::arch::x86_64::paging::PageTable);
        let kernel_table = &*(kernel_va as *const crate::arch::x86_64::paging::PageTable);

        for index in 256..512 {
            if user_table.entries[index].raw() != kernel_table.entries[index].raw() {
                crate::drivers::serial::print("CR3 TEST: kernel-half mismatch at ");
                crate::drivers::serial::print_u64(index as u64);
                crate::drivers::serial::println("");
                return;
            }

            if user_table.entries[index].is_present() {
                present += 1;
            }
        }
    }

    if present == 0 {
        crate::drivers::serial::println("CR3 TEST: kernel mappings MISSING");
        return;
    }

    crate::drivers::serial::print("Kernel-half identical, present entries: ");
    crate::drivers::serial::print_u64(present as u64);
    crate::drivers::serial::println("");

    let canary: u64 = 0xC3A0_1234_5678_9ABC;

    unsafe {
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));

        crate::arch::x86_64::paging::load_cr3(pml4_phys);

        let loaded = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

        if loaded != pml4_phys {
            crate::arch::x86_64::paging::load_cr3(kernel_cr3);
            core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
            crate::drivers::serial::println("CR3 TEST: load/readback FAILED");
            return;
        }

        let mut probe: u64 = 0;
        core::ptr::write_volatile(&mut probe, canary);
        let check = core::ptr::read_volatile(&probe);

        if check != canary {
            crate::arch::x86_64::paging::load_cr3(kernel_cr3);
            core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
            crate::drivers::serial::println("CR3 TEST: stack probe FAILED");
            return;
        }

        crate::arch::x86_64::paging::load_cr3(kernel_cr3);

        let restored = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));

        if restored != kernel_cr3 {
            crate::drivers::serial::println("CR3 TEST: restore FAILED");
            return;
        }
    }

    crate::drivers::serial::println("CR3 load/readback/restore: OK");
    crate::drivers::serial::println("USER PROCESS CR3 TEST: PASS");
}
