use crate::arch::x86_64::paging::PageTable;
use crate::memory::physical::{FrameAllocator, PhysFrame};

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
    crate::drivers::serial::println("Virtual memory test");

    let frame_allocator = FrameAllocator::new();

    let mut virtual_memory = match super::virtualm::VirtualMemoryManager::new(frame_allocator) {
        Some(vm) => vm,

        None => {
            crate::drivers::serial::println("ERROR: failed to create virtual memory manager");

            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    };

    crate::drivers::serial::println("Virtual memory manager created");

    let pml4 = virtual_memory.pml4();

    crate::drivers::serial::print("PML4 address: 0x");

    crate::drivers::serial::print_u64(pml4.address);

    crate::drivers::serial::println("");

    if virtual_memory.map_identity_2mb(512 * 1024 * 1024) {
        crate::drivers::serial::println("Identity map: 512 MiB");

        crate::drivers::serial::println("PML4 -> PDPT -> PD: OK");
    } else {
        crate::drivers::serial::println("ERROR: failed to create identity map");
    }

    if virtual_memory.map_hhdm() {
        crate::drivers::serial::println("HHDM mapping: OK");
    } else {
        crate::drivers::serial::println("ERROR: HHDM mapping failed");
    }

    if virtual_memory.map_kernel() {
        crate::drivers::serial::println("Kernel higher-half mapping: OK");
    } else {
        crate::drivers::serial::println("ERROR: kernel mapping failed");
    }

    {
        let lapic_phys_raw = crate::arch::x86_64::smp::acpi::lapic_phys();

        let lapic_phys = if lapic_phys_raw != 0 {
            lapic_phys_raw
        } else {
            0xFEE0_0000
        };

        if !virtual_memory.map_mmio(lapic_phys, crate::arch::x86_64::lapic::LAPIC_SIZE) {
            crate::drivers::serial::println("SMP: LAPIC MMIO mapping FAILED");
        } else {
            crate::drivers::serial::println("SMP: LAPIC MMIO mapping OK");
        }
    }

    let test_virtual = 0x0000_4000_0000_0000u64;

    let test_physical = 0x0040_0000u64;

    if virtual_memory.map_page(test_virtual, test_physical, true, false) {
        crate::drivers::serial::println("4 KiB page mapping: OK");
    } else {
        crate::drivers::serial::println("ERROR: 4 KiB page mapping failed");
    }

    let stack_top = match virtual_memory.create_bootstrap_stack() {
        Some(address) => {
            crate::drivers::serial::println("Bootstrap stack: OK");

            crate::drivers::serial::print("Stack top: 0x");

            crate::drivers::serial::print_u64(address);

            crate::drivers::serial::println("");

            address
        }

        None => {
            crate::drivers::serial::println("ERROR: failed to create bootstrap stack");

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
                crate::drivers::serial::print("AP stack ");
                crate::drivers::serial::print_u64((index + 1) as u64);
                crate::drivers::serial::print(": 0x");
                crate::drivers::serial::print_u64(stack_top);
                crate::drivers::serial::println("");

                stack_top
            }

            None => {
                crate::drivers::serial::println("ERROR: failed to create AP stack");

                loop {
                    unsafe {
                        core::arch::asm!("hlt");
                    }
                }
            }
        };
    }

    crate::drivers::serial::println("Entering new address space...");

    if virtual_memory.map_heap() {
        crate::drivers::serial::println("Kernel heap mapping: OK");
    } else {
        crate::drivers::serial::println("ERROR: kernel heap mapping failed");

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

extern "C" fn after_address_space_switch() -> ! {
    crate::drivers::serial::println("Entered new address space!");

    let cr3 = crate::arch::x86_64::paging::read_cr3();

    crate::drivers::serial::print("New CR3: 0x");
    crate::drivers::serial::print_u64(cr3);
    crate::drivers::serial::println("");

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

        crate::drivers::serial::print("APIC MSR before enable: 0x");
        crate::drivers::serial::print_hex_u64(apic_msr_before);
        crate::drivers::serial::println("");

        crate::arch::x86_64::io::wrmsr(IA32_APIC_BASE, apic_msr_before | APIC_ENABLE);

        let apic_msr_after = crate::arch::x86_64::io::rdmsr(IA32_APIC_BASE);

        crate::drivers::serial::print("APIC MSR after enable: 0x");
        crate::drivers::serial::print_hex_u64(apic_msr_after);
        crate::drivers::serial::println("");
    }

    crate::arch::x86_64::lapic::init(lapic_phys);

    let bsp_apic_id = crate::arch::x86_64::lapic::id();

    crate::drivers::serial::print("LAPIC ID after switch: ");
    crate::drivers::serial::print_u64(bsp_apic_id as u64);
    crate::drivers::serial::println("");

    crate::arch::x86_64::smp::percpu::init_bsp(bsp_apic_id);

    crate::arch::x86_64::lapic::timer_init();
    crate::arch::x86_64::lapic::timer_start(10_000_000);

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

    crate::drivers::serial::print("RFLAGS.IF (1=enabled): ");
    crate::drivers::serial::print_u64((rflags >> 9) & 1);
    crate::drivers::serial::println("");

    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::serial::println("STI: interrupts enabled");

    crate::memory::heap::KERNEL_HEAP.init();

    crate::drivers::serial::println("Kernel heap initialized");

    let ap_entry = crate::arch::x86_64::smp::trampoline::ap_main as *const () as usize as u64;

    let aps = [
        (1u32, AP1_STACK_TOP),
        (2u32, AP2_STACK_TOP),
        (3u32, AP3_STACK_TOP),
    ];

    for &(apic_id, stack_top) in &aps {
        crate::drivers::serial::println("--------------------------------");

        crate::drivers::serial::print("SMP: preparing APIC ");
        crate::drivers::serial::print_u64(apic_id as u64);
        crate::drivers::serial::println("");

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
            crate::drivers::serial::print("SMP: APIC ");
            crate::drivers::serial::print_u64(apic_id as u64);
            crate::drivers::serial::println(" started successfully");
        } else {
            crate::drivers::serial::print("ERROR: APIC ");
            crate::drivers::serial::print_u64(apic_id as u64);
            crate::drivers::serial::println(" failed to start");
        }
    }

    let online = crate::arch::x86_64::smp::percpu::online_count();

    crate::drivers::serial::print("SMP: CPUs online = ");
    crate::drivers::serial::print_u64(online as u64);
    crate::drivers::serial::println("");

    let layout = match core::alloc::Layout::from_size_align(128, 8) {
        Ok(layout) => layout,

        Err(_) => {
            crate::drivers::serial::println("ERROR: invalid layout");

            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    };

    let bsp = crate::arch::x86_64::smp::percpu::current();

    crate::drivers::serial::print("SMP: current CPU = ");
    crate::drivers::serial::print_u64(bsp.cpu_id as u64);
    crate::drivers::serial::println("");

    let ptr = unsafe { core::alloc::GlobalAlloc::alloc(&crate::memory::heap::KERNEL_HEAP, layout) };

    if ptr.is_null() {
        crate::drivers::serial::println("ERROR: heap allocation failed");
    } else {
        crate::drivers::serial::println("Heap allocation: OK");

        unsafe {
            *ptr = 0x42;
        }

        if unsafe { *ptr } == 0x42 {
            crate::drivers::serial::println("Heap write/read: OK");
        } else {
            crate::drivers::serial::println("ERROR: heap write/read failed");
        }
    }

    let value = alloc::boxed::Box::new(12345u64);

    if *value == 12345 {
        crate::drivers::serial::println("Box allocation: OK");
    } else {
        crate::drivers::serial::println("ERROR: Box allocation failed");
    }

    crate::task::test::scheduler_test();

    // loop {
    //     unsafe {
    //         core::arch::asm!("hlt");
    //     }
    // }
}
