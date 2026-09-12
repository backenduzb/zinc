use crate::arch::x86_64::paging::PageTable;
use crate::memory::physical::{FrameAllocator, PhysFrame};

const HUGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;
const STACK_BASE: u64 = 0xFFFF_FFFF_A000_0000;
const STACK_SIZE: u64 = 64 * 1024;
const PAGE_SIZE: u64 = 4096;

unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;

    pub fn enter_new_address_space(
        pml4: u64,
        stack_top: u64,
        entry: u64,
    ) -> !;
}

pub fn virtual_memory_test() -> ! {
    crate::drivers::serial::println("Virtual memory test");

    let frame_allocator = FrameAllocator::new();

    let mut virtual_memory =
        match super::virtualm::VirtualMemoryManager::new(frame_allocator) {
            Some(vm) => vm,

            None => {
                crate::drivers::serial::println(
                    "ERROR: failed to create virtual memory manager",
                );

                loop {
                    unsafe {
                        core::arch::asm!("hlt");
                    }
                }
            }
        };

    crate::drivers::serial::println(
        "Virtual memory manager created",
    );

    let pml4 = virtual_memory.pml4();

    crate::drivers::serial::print(
        "PML4 address: 0x",
    );

    crate::drivers::serial::print_u64(
        pml4.address,
    );

    crate::drivers::serial::println("");

    if virtual_memory.map_identity_2mb(
        512 * 1024 * 1024,
    ) {
        crate::drivers::serial::println(
            "Identity map: 512 MiB",
        );

        crate::drivers::serial::println(
            "PML4 -> PDPT -> PD: OK",
        );
    } else {
        crate::drivers::serial::println(
            "ERROR: failed to create identity map",
        );
    }

    if virtual_memory.map_hhdm() {
        crate::drivers::serial::println(
            "HHDM mapping: OK",
        );
    } else {
        crate::drivers::serial::println(
            "ERROR: HHDM mapping failed",
        );
    }

    if virtual_memory.map_kernel() {
        crate::drivers::serial::println(
            "Kernel higher-half mapping: OK",
        );
    } else {
        crate::drivers::serial::println(
            "ERROR: kernel mapping failed",
        );
    }

    let test_virtual =
        0x0000_4000_0000_0000u64;

    let test_physical =
        0x0040_0000u64;

    if virtual_memory.map_page(
        test_virtual,
        test_physical,
        true,
        false,
    ) {
        crate::drivers::serial::println(
            "4 KiB page mapping: OK",
        );
    } else {
        crate::drivers::serial::println(
            "ERROR: 4 KiB page mapping failed",
        );
    }

    let stack_top =
        match virtual_memory.create_bootstrap_stack() {
            Some(address) => {
                crate::drivers::serial::println(
                    "Bootstrap stack: OK",
                );

                crate::drivers::serial::print(
                    "Stack top: 0x",
                );

                crate::drivers::serial::print_u64(
                    address,
                );

                crate::drivers::serial::println("");

                address
            }

            None => {
                crate::drivers::serial::println(
                    "ERROR: failed to create bootstrap stack",
                );

                loop {
                    unsafe {
                        core::arch::asm!("hlt");
                    }
                }
            }
        };

    crate::drivers::serial::println(
        "Entering new address space...",
    );

    if virtual_memory.map_heap() {
        crate::drivers::serial::println(
            "Kernel heap mapping: OK",
        );
    } else {
        crate::drivers::serial::println(
            "ERROR: kernel heap mapping failed",
        );

        loop {
            unsafe {
                core::arch::asm!("hlt");
            }
        }
    }

    let entry =
        after_address_space_switch as usize as u64;

    unsafe {
        enter_new_address_space(
            pml4.address,
            stack_top,
            entry,
        );
    }
}

extern "C" fn after_address_space_switch() -> ! {
    crate::drivers::serial::println(
        "Entered new address space!",
    );

    let cr3 =
        crate::arch::x86_64::paging::read_cr3();

    crate::drivers::serial::print(
        "New CR3: 0x",
    );

    crate::drivers::serial::print_u64(cr3);

    crate::drivers::serial::println("");


    crate::memory::heap::KERNEL_HEAP.init();

    crate::drivers::serial::println(
        "Kernel heap initialized",
    );

    let layout =
        match core::alloc::Layout::from_size_align(
            128,
            8,
        ) {
            Ok(layout) => layout,

            Err(_) => {
                crate::drivers::serial::println(
                    "ERROR: invalid layout",
                );

                loop {
                    unsafe {
                        core::arch::asm!("hlt");
                    }
                }
            }
        };

    let ptr = unsafe {
        core::alloc::GlobalAlloc::alloc(
            &crate::memory::heap::KERNEL_HEAP,
            layout,
        )
    };

    if ptr.is_null() {
        crate::drivers::serial::println(
            "ERROR: heap allocation failed",
        );
    } else {
        crate::drivers::serial::println(
            "Heap allocation: OK",
        );

        unsafe {
            *ptr = 0x42;
        }

        if unsafe { *ptr } == 0x42 {
            crate::drivers::serial::println(
                "Heap write/read: OK",
            );
        } else {
            crate::drivers::serial::println(
                "ERROR: heap write/read failed",
            );
        }
    }

    let value =
        alloc::boxed::Box::new(12345u64);

    if *value == 12345 {
        crate::drivers::serial::println(
            "Box allocation: OK",
        );
    } else {
        crate::drivers::serial::println(
            "ERROR: Box allocation failed",
        );
    }

    crate::task::test::scheduler_test();
}