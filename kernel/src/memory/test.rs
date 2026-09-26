use crate::arch::x86_64::paging::PageTable;
use crate::memory::address_space::AddressSpace;
use crate::memory::physical::{FrameAllocator, PhysFrame};
use crate::memory::user_stack::{USER_STACK_PAGES, UserStack};

const AP1_STACK_TOP: u64 = 0xFFFF_FFFF_A010_FFF8;
const AP2_STACK_TOP: u64 = 0xFFFF_FFFF_A011_FFF8;
const AP3_STACK_TOP: u64 = 0xFFFF_FFFF_A012_FFF8;
const HUGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;
const STACK_BASE: u64 = 0xFFFF_FFFF_A000_0000;
const STACK_SIZE: u64 = 64 * 1024;
const PAGE_SIZE: u64 = 4096;

unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;

    pub fn enter_new_address_space(pml4: u64, stack_top: u64, entry: u64) -> !;
}

pub fn virtual_memory_test() -> ! {
    crate::log_test!("Virtual memory test");

    let frame_allocator = FrameAllocator::new();

    let mut virtual_memory = match super::virtualm::VirtualMemoryManager::new(frame_allocator) {
        Some(vm) => vm,

        None => {
            crate::log_test!("ERROR: failed to create virtual memory manager");

            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    };

    crate::log_boot!("Virtual memory manager created");

    let pml4 = virtual_memory.pml4();

    crate::log_boot_raw!("PML4 address: 0x");

    crate::log_boot_raw!("{}", pml4.address);

    crate::log_boot!("");

    if virtual_memory.map_identity_2mb(512 * 1024 * 1024) {
        crate::log_boot!("Identity map: 512 MiB");

        crate::log_boot!("PML4 -> PDPT -> PD: OK");
    } else {
        crate::log_test!("ERROR: failed to create identity map");
    }

    if virtual_memory.map_hhdm() {
        crate::log_boot!("HHDM mapping: OK");
    } else {
        crate::log_boot!("ERROR: HHDM mapping failed");
    }

    if virtual_memory.map_kernel() {
        crate::log_boot!("Kernel higher-half mapping: OK");
    } else {
        crate::log_test!("ERROR: kernel mapping failed");
    }

    {
        let lapic_phys_raw = crate::arch::x86_64::smp::acpi::lapic_phys();

        let lapic_phys = if lapic_phys_raw != 0 {
            lapic_phys_raw
        } else {
            0xFEE0_0000
        };

        if !virtual_memory.map_mmio(lapic_phys, crate::arch::x86_64::lapic::LAPIC_SIZE) {
            crate::log_smp!("SMP: LAPIC MMIO mapping FAILED");
        } else {
            crate::log_smp!("SMP: LAPIC MMIO mapping OK");
        }
    }

    let test_virtual = 0x0000_4000_0000_0000u64;

    let test_physical = 0x0040_0000u64;

    if virtual_memory.map_page(test_virtual, test_physical, true, false) {
        crate::log_boot!("4 KiB page mapping: OK");
    } else {
        crate::log_test!("ERROR: 4 KiB page mapping failed");
    }

    let stack_top = match virtual_memory.create_bootstrap_stack() {
        Some(address) => {
            crate::log_boot!("Bootstrap stack: OK");

            crate::log_boot_raw!("Stack top: 0x");

            crate::log_boot_raw!("{}", address);

            crate::log_boot!("");

            address
        }

        None => {
            crate::log_boot!("ERROR: failed to create bootstrap stack");

            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    };

    const AP_STACK_BASE: u64 = 0xFFFF_FFFF_A010_0000;
    const AP_STACK_SIZE: u64 = 64 * 1024;

    let mut ap_stack_tops = [0u64; 3];

    for index in 0..3 {
        let base = AP_STACK_BASE + index as u64 * AP_STACK_SIZE;

        ap_stack_tops[index] = match virtual_memory.create_ap_stack(base, AP_STACK_SIZE) {
            Some(stack_top) => {
                crate::log_boot_raw!("AP stack ");
                crate::log_boot_raw!("{}", (index + 1) as u64);
                crate::log_boot_raw!(": 0x");
                crate::log_boot_raw!("{}", stack_top);
                crate::log_boot!("");

                stack_top
            }

            None => {
                crate::log_test!("ERROR: failed to create AP stack");

                loop {
                    unsafe {
                        core::arch::asm!("hlt");
                    }
                }
            }
        };
    }

    crate::log_boot!("Entering new address space...");

    if virtual_memory.map_heap() {
        crate::log_boot!("Kernel heap mapping: OK");
    } else {
        crate::log_test!("ERROR: kernel heap mapping failed");

        loop {
            unsafe {
                core::arch::asm!("hlt");
            }
        }
    }

    let entry = after_address_space_switch as usize as u64;

    unsafe {
        enter_new_address_space(pml4.address, stack_top, entry);
    }
}

use core::sync::atomic::{AtomicBool, Ordering};

/// Dump CPU3 scheduler slot occupancy for diagnosing "scheduler full".
/// Prints slot / TID / PID / ThreadState / cpu_id per occupied slot.
fn log_cpu3_slots(reason: &str) {
    crate::log_smp_raw!("CPU3 SLOTS (");
    crate::log_boot_raw!("{}", reason);
    crate::log_boot!(")");
    unsafe {
        for slot in 0..8 {
            match crate::task::SCHEDULERS[3].thread(slot) {
                Some(t) => {
                    let state = match t.state() {
                        crate::task::thread::ThreadState::Ready => "Ready",
                        crate::task::thread::ThreadState::Running => "Running",
                        crate::task::thread::ThreadState::Waiting => "Waiting",
                        crate::task::thread::ThreadState::Sleeping => "Sleeping",
                        crate::task::thread::ThreadState::Terminated => "Terminated",
                    };
                    crate::log_boot_raw!("  slot=");
                    crate::log_boot_raw!("{}", slot as u64);
                    crate::log_boot_raw!(" TID=");
                    crate::log_boot_raw!("{}", t.id());
                    crate::log_boot_raw!(" PID=");
                    crate::log_boot_raw!("{}", t.process_id());
                    crate::log_boot_raw!(" state=");
                    crate::log_boot!("{}", state);
                    crate::log_smp_raw!("  cpu_id=");
                    crate::log_boot_raw!("{}", t.cpu_id() as u64);
                    crate::log_boot!("");
                }
                None => {
                    crate::log_boot_raw!("  slot=");
                    crate::log_boot_raw!("{}", slot as u64);
                    crate::log_boot!(" empty");
                }
            }
        }
        crate::log_boot_raw!("  count=");
        crate::log_boot_raw!("{}", crate::task::SCHEDULERS[3].count() as u64);
        crate::log_boot!("");
    }
}

/// Wait until the CPU3 test's reaper signals completion.
///
/// The reaper sets its flag only after the full lifecycle finished:
/// process terminated -> ProcessManager::reap() -> AddressSpace
/// destroyed -> process removed -> completion=true -> task::exit().
///
/// Intentionally spin-based with NO hlt/sleep here. This runs on the
/// BSP before its scheduler starts, so `crate::task::sleep` is a no-op
/// and a missed hlt wakeup would deadlock boot. APs run on their own
/// vCPUs, so a bounded pause-spin cannot deadlock them. Bounded +
/// fail-open: on timeout dumps CPU3 slots and returns so boot always
/// continues. Only atomics are polled (no cross-CPU struct walks).
fn wait_for_vfs_complete(flag: &AtomicBool, name: &str) {
    crate::log_vfs_raw!("VFS EXEC WAIT: ");
    crate::log_boot!("{}", name);

    const INNER_SPINS: u64 = 2048;
    const HEARTBEAT_OUTER: u64 = 20000;
    const TIMEOUT_OUTER: u64 = 120_000;

    let mut outer = 0u64;
    let done = loop {
        for _ in 0..INNER_SPINS {
            core::hint::spin_loop();
        }
        outer += 1;
        if flag.load(Ordering::Acquire) {
            break true;
        }
        if outer % HEARTBEAT_OUTER == 0 {
            crate::log_vfs_raw!("VFS EXEC WAIT ALIVE: ");
            crate::log_boot_raw!("{}", name);
            crate::log_sched_raw!(" ticks=");
            crate::log_boot_raw!("{}", crate::arch::x86_64::lapic::ticks());
            crate::log_boot!("");
        }
        if outer >= TIMEOUT_OUTER {
            break false;
        }
    };

    if done {
        crate::log_vfs_raw!("VFS EXEC WAIT DONE: ");
        crate::log_boot!("{}", name);
    } else {
        crate::log_vfs_raw!("VFS EXEC WAIT TIMEOUT: ");
        crate::log_boot!("{}", name);
        log_cpu3_slots(name);
    }
}

extern "C" fn after_address_space_switch() -> ! {
    crate::log_boot!("Entered new address space!");

    let cr3 = crate::arch::x86_64::paging::read_cr3();

    crate::log_cr3_raw!("New CR3: 0x");
    crate::log_boot_raw!("{}", cr3);
    crate::log_boot!("");

    let lapic_phys_raw = crate::arch::x86_64::smp::acpi::lapic_phys();

    let lapic_phys = if lapic_phys_raw != 0 {
        lapic_phys_raw
    } else {
        0xFEE0_0000
    };

    unsafe {
        const IA32_APIC_BASE: u32 = 0x1B;
        const APIC_ENABLE: u64 = 1 << 11;

        let apic_msr_before = crate::arch::x86_64::io::rdmsr(IA32_APIC_BASE);

        crate::log_smp_raw!("APIC MSR before enable: 0x");
        crate::log_boot_raw!("{:016X}", apic_msr_before);
        crate::log_boot!("");

        crate::arch::x86_64::io::wrmsr(IA32_APIC_BASE, apic_msr_before | APIC_ENABLE);

        let apic_msr_after = crate::arch::x86_64::io::rdmsr(IA32_APIC_BASE);

        crate::log_smp_raw!("APIC MSR after enable: 0x");
        crate::log_boot_raw!("{:016X}", apic_msr_after);
        crate::log_boot!("");
    }

    crate::arch::x86_64::lapic::init(lapic_phys);

    let bsp_apic_id = crate::arch::x86_64::lapic::id();

    crate::log_smp_raw!("LAPIC ID after switch: ");
    crate::log_boot_raw!("{}", bsp_apic_id as u64);
    crate::log_boot!("");

    crate::arch::x86_64::smp::percpu::init_bsp(bsp_apic_id);

    crate::arch::x86_64::lapic::timer_init();
    crate::arch::x86_64::lapic::timer_start(crate::arch::x86_64::lapic::TIMER_INITIAL_COUNT);

    crate::arch::x86_64::lapic::timer_debug_count();

    for _ in 0..1_000_000 {
        core::hint::spin_loop();
    }

    crate::arch::x86_64::lapic::timer_debug_count();

    let rflags: u64;

    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            out(reg) rflags,
            options(nomem, preserves_flags)
        );
    }

    crate::log_boot_raw!("RFLAGS.IF (1=enabled): ");
    crate::log_boot_raw!("{}", (rflags >> 9) & 1);
    crate::log_boot!("");

    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::log_boot!("STI: interrupts enabled");

    crate::memory::heap::KERNEL_HEAP.init();

    crate::log_boot!("Kernel heap initialized");
    crate::log_boot_raw!("Heap remaining: ");
    crate::log_boot_raw!("{}", crate::memory::heap::KERNEL_HEAP.remaining() as u64);
    crate::log_boot!(" bytes");

    crate::drivers::storage::tests::test_create();

    crate::drivers::storage::tests::test_vfs_elf();

    let ap_entry = crate::arch::x86_64::smp::trampoline::ap_main as *const () as usize as u64;

    let aps = [
        (1u32, AP1_STACK_TOP),
        (2u32, AP2_STACK_TOP),
        (3u32, AP3_STACK_TOP),
    ];

    for &(apic_id, stack_top) in &aps {
        crate::log_boot!("--------------------------------");

        crate::log_smp_raw!("SMP: preparing APIC ");
        crate::log_boot_raw!("{}", apic_id as u64);
        crate::log_boot!("");

        crate::arch::x86_64::smp::trampoline::reset_ap_state(apic_id as usize);

        crate::arch::x86_64::smp::trampoline::install(cr3, stack_top, ap_entry);

        crate::arch::x86_64::lapic::start_ap(apic_id, 0x08);

        let mut started = false;

        for _ in 0..10_000_000 {
            if crate::arch::x86_64::smp::trampoline::is_ap_started(apic_id as usize) {
                started = true;
                break;
            }

            core::hint::spin_loop();
        }

        if started {
            crate::log_smp_raw!("SMP: APIC ");
            crate::log_boot_raw!("{}", apic_id as u64);
            crate::log_boot!(" started successfully");
        } else {
            crate::log_smp_raw!("ERROR: APIC ");
            crate::log_boot_raw!("{}", apic_id as u64);
            crate::log_test!(" failed to start");
        }
    }

    let online = crate::arch::x86_64::smp::percpu::online_count();

    crate::log_smp_raw!("SMP: CPUs online = ");
    crate::log_boot_raw!("{}", online as u64);
    crate::log_boot!("");

    let kernel_created = unsafe { (*crate::task::process_manager()).create_with_pid(0) };

    if !kernel_created {
        crate::log_test!("ERROR: failed to create kernel process");
    } else {
        crate::log_boot!("Process manager: initialized");

        crate::log_boot!("Kernel process: PID=0");

        unsafe {
            let pm = crate::task::process_manager();

            let beta = (*pm).add_thread(0, crate::task::test::AP_BETA_ID);

            let silf = (*pm).add_thread(0, crate::task::test::AP_SILF_ID);

            let gamma = (*pm).add_thread(0, crate::task::test::AP_GAMMA_ID);

            if beta && silf && gamma {
                crate::log_vfs!("AP threads registered to kernel process");
            } else {
                crate::log_vfs!("ERROR: failed to register AP threads");

                if !beta {
                    crate::log_test!("  AP_BETA registration failed");
                }

                if !silf {
                    crate::log_test!("  AP_SILF registration failed");
                }

                if !gamma {
                    crate::log_test!("  AP_GAMMA registration failed");
                }
            }
        }
    }

    let layout = match core::alloc::Layout::from_size_align(128, 8) {
        Ok(layout) => layout,

        Err(_) => {
            crate::log_boot!("ERROR: invalid layout");

            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    };

    let bsp = crate::arch::x86_64::smp::percpu::current();

    crate::log_smp_raw!("SMP: current CPU = ");

    crate::log_boot_raw!("{}", bsp.cpu_id as u64);

    crate::log_boot!("");

    let ptr = unsafe { core::alloc::GlobalAlloc::alloc(&crate::memory::heap::KERNEL_HEAP, layout) };

    if ptr.is_null() {
        crate::log_test!("ERROR: heap allocation failed");
    } else {
        crate::log_boot!("Heap allocation: OK");

        unsafe {
            *ptr = 0x42;
        }

        if unsafe { *ptr } == 0x42 {
            crate::log_vfs!("Heap write/read: OK");
        } else {
            crate::log_vfs!("ERROR: heap write/read failed");
        }
    }

    let value = alloc::boxed::Box::new(12345u64);

    if *value == 12345 {
        crate::log_boot!("Box allocation: OK");
    } else {
        crate::log_test!("ERROR: Box allocation failed");
    }

    crate::task::test::process_address_space_test();

    crate::task::test::user_thread_test();

    crate::task::test::user_process_cr3_test();

    crate::exec::test::test_loader();

    crate::exec::test::test_process_reap();

    let exec_runtime_ok = crate::exec::test::test_exec_runtime();
    if exec_runtime_ok {
        crate::log_vfs!("EXEC RUNTIME: ELF thread queued");
    } else {
        crate::log_exec!("EXEC RUNTIME: FAILED, fallback to manual ring3");
    }

    if crate::exec::vfs::test_hello_exec() {
        crate::log_vfs!("VFS EXEC: /bin/hello queued for Ring3");
    } else {
        crate::log_vfs!("VFS EXEC: FAILED");
    }

    if crate::exec::test::test_init_exec() {
        crate::log_vfs!("VFS EXEC: /bin/init queued for Ring3 (boot path)");
    } else {
        crate::log_vfs!("VFS EXEC INIT: FAILED");
    }

    if crate::exec::test::test_badptr_exec() {
        crate::log_vfs!("VFS EXEC: /bin/badptr queued for Ring3");
        wait_for_vfs_complete(&crate::exec::vfs::BADPTR_COMPLETE, "badptr");
    } else {
        crate::log_vfs!("VFS EXEC BADPTR: FAILED");
    }

    if !crate::exec::vfs::test_open_test_exec() {
        crate::log_vfs!("VFS EXEC OPEN TEST: FAILED");
    } else {
        wait_for_vfs_complete(&crate::exec::vfs::OPEN_TEST_COMPLETE, "open_test");
    }

    if !crate::exec::vfs::test_read_test_exec() {
        crate::log_vfs!("VFS EXEC READ TEST: FAILED");
    } else {
        wait_for_vfs_complete(&crate::exec::vfs::READ_TEST_COMPLETE, "read_test");
    }

    if !crate::exec::vfs::test_close_test_exec() {
        crate::log_vfs!("VFS EXEC CLOSE TEST: FAILED");
    } else {
        wait_for_vfs_complete(&crate::exec::vfs::CLOSE_TEST_COMPLETE, "close_test");
    }

    if !crate::exec::vfs::test_execve_exec() {
        crate::log_vfs!("VFS EXEC EXECVE TEST: FAILED");
    } else {
        wait_for_vfs_complete(&crate::exec::vfs::EXECVE_TEST_COMPLETE, "execve_test");
    }

    crate::task::ring3::prepare_ring3_user_thread();
    crate::task::ring3::prepare_wait_blocking_test();

    crate::task::test::scheduler_test();
}

pub fn address_space_test() {
    crate::log_test!("=== ADDRESS SPACE TEST ===");

    let mut allocator = crate::memory::physical::FrameAllocator::new();

    let Some(address_space) = crate::memory::address_space::AddressSpace::new(&mut allocator)
    else {
        crate::log_test!("AddressSpace: FAILED");
        return;
    };

    crate::log_boot_raw!("AddressSpace PML4: 0x");
    crate::log_boot_raw!("{}", address_space.pml4().address);
    crate::log_boot!("");

    crate::log_boot!("AddressSpace: OK");
}

pub fn user_address_space_test() {
    crate::log_ring3!("=== USER ADDRESS SPACE TEST ===");

    let mut allocator = crate::memory::physical::FrameAllocator::new();

    let Some(address_space) = crate::memory::address_space::AddressSpace::new_user(&mut allocator)
    else {
        crate::log_ring3!("User AddressSpace: FAILED");
        return;
    };

    crate::log_ring3_raw!("User PML4: 0x");
    crate::log_boot_raw!("{}", address_space.pml4().address);
    crate::log_boot!("");

    crate::log_ring3!("User AddressSpace: OK");
}

pub fn user_page_map_test() {
    crate::log_ring3!("=== USER PAGE MAP TEST ===");

    let mut allocator = crate::memory::physical::FrameAllocator::new();

    let Some(mut address_space) =
        crate::memory::address_space::AddressSpace::new_user(&mut allocator)
    else {
        crate::log_ring3!("User AddressSpace: FAILED");
        return;
    };

    let Some(frame) = allocator.alloc_frame() else {
        crate::log_ring3!("User page frame allocation: FAILED");
        return;
    };

    crate::log_ring3_raw!("User frame: 0x");
    crate::log_boot_raw!("{}", frame.address);
    crate::log_boot!("");

    let virtual_address = 0x0040_0000;

    if address_space.map_page(&mut allocator, virtual_address, frame.address, true, true) {
        crate::log_ring3!("User page mapping: OK");
    } else {
        crate::log_ring3!("User page mapping: FAILED");
    }
}

pub fn user_page_unmap_test() {
    crate::log_ring3!("=== USER PAGE UNMAP TEST ===");

    let mut allocator = crate::memory::physical::FrameAllocator::new();

    let Some(mut address_space) =
        crate::memory::address_space::AddressSpace::new_user(&mut allocator)
    else {
        crate::log_ring3!("User AddressSpace: FAILED");
        return;
    };

    let Some(frame) = allocator.alloc_frame() else {
        crate::log_test!("Frame allocation: FAILED");
        return;
    };

    let virtual_address = 0x0040_0000;

    if !address_space.map_page(&mut allocator, virtual_address, frame.address, true, true) {
        crate::log_test!("Mapping: FAILED");
        return;
    }

    crate::log_boot!("Mapping: OK");

    match address_space.translate_page(virtual_address) {
        Some(address) if address == frame.address => {
            crate::log_boot!("Translation: OK");
        }

        Some(address) => {
            crate::log_boot_raw!("Translation: WRONG 0x");
            crate::log_boot_raw!("{}", address);
            crate::log_boot!("");
            return;
        }

        None => {
            crate::log_test!("Translation: FAILED");
            return;
        }
    }

    let unmapped = address_space.unmap_page(&mut allocator, virtual_address);

    match unmapped {
        Some(unmapped_frame) if unmapped_frame.address == frame.address => {
            crate::log_boot!("Unmap: OK");
        }

        _ => {
            crate::log_test!("Unmap: FAILED");
            return;
        }
    }

    if address_space.translate_page(virtual_address).is_none() {
        crate::log_boot!("Translation after unmap: OK");
    } else {
        crate::log_test!("Translation after unmap: FAILED");
    }
}

pub fn nx_test() {
    crate::log_test!("=== NX PAGE TEST ===");

    let mut allocator = crate::memory::physical::FrameAllocator::new();

    let Some(mut address_space) =
        crate::memory::address_space::AddressSpace::new_user(&mut allocator)
    else {
        crate::log_test!("NX AddressSpace: FAILED");
        return;
    };

    let Some(frame) = allocator.alloc_frame() else {
        crate::log_test!("NX frame allocation: FAILED");
        return;
    };

    let va = 0x0040_0000;

    if !address_space.map_page(&mut allocator, va, frame.address, false, true) {
        crate::log_test!("NX mapping: FAILED");
        return;
    }

    match address_space.translate_page(va) {
        Some(address) if address == frame.address => {
            crate::log_boot!("NX mapping: OK");
        }

        _ => {
            crate::log_test!("NX mapping translation: FAILED");
        }
    }

    if let Some(unmapped) = address_space.unmap_page(&mut allocator, va) {
        if unmapped.address == frame.address {
            crate::log_boot!("NX cleanup: OK");
        }
    }
}

pub fn user_stack_test() {
    crate::log_ring3!("=== USER STACK TEST ===");

    let mut allocator = FrameAllocator::new();

    let Some(mut address_space) = AddressSpace::new_user(&mut allocator) else {
        crate::log_ring3!("USER STACK: AddressSpace FAILED");
        return;
    };

    let Some(stack) = UserStack::map(&mut address_space, &mut allocator) else {
        crate::log_ring3!("USER STACK: MAP FAILED");
        return;
    };

    crate::log_boot_raw!("Stack base: 0x");
    crate::log_boot_raw!("{}", stack.base);
    crate::log_boot!("");

    crate::log_boot_raw!("Stack top: 0x");
    crate::log_boot_raw!("{}", stack.top);
    crate::log_boot!("");

    crate::log_boot_raw!("Stack size: ");
    crate::log_boot_raw!("{}", stack.size);
    crate::log_boot!("");

    let mut passed = true;

    for i in 0..USER_STACK_PAGES {
        let va = stack.base + (i as u64 * PAGE_SIZE);

        if address_space.translate_page(va).is_some() {
            crate::log_boot_raw!("STACK page ");
            crate::log_boot_raw!("{}", i as u64);
            crate::log_boot!(": mapped");
        } else {
            passed = false;

            crate::log_boot_raw!("STACK page ");
            crate::log_boot_raw!("{}", i as u64);
            crate::log_test!(": FAILED");
        }
    }

    if address_space.translate_page(stack.top).is_none() {
        crate::log_boot!("STACK top boundary: OK");
    } else {
        passed = false;

        crate::log_test!("STACK top boundary: FAILED");
    }

    if passed {
        crate::log_ring3!("USER STACK TEST: PASS");
    } else {
        crate::log_ring3!("USER STACK TEST: FAIL");
    }
}
