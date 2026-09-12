use crate::arch::x86_64::paging::PageTable;
use crate::memory::physical::{FrameAllocator, PhysFrame};

const HUGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;
const STACK_BASE: u64 = 0xFFFF_FFFF_A000_0000;
const STACK_SIZE: u64 = 64 * 1024;
const PAGE_SIZE: u64 = 4096;

unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;
}

pub struct VirtualMemoryManager {
    frame_allocator: FrameAllocator,
    pml4: PhysFrame,
}

unsafe extern "C" {
    pub fn switch_address_space(pml4: u64, stack_top: u64);

    pub fn enter_new_address_space(pml4: u64, stack_top: u64, entry: u64) -> !;
}

impl VirtualMemoryManager {
    pub fn new(mut frame_allocator: FrameAllocator) -> Option<Self> {
        let pml4 = frame_allocator.alloc_frame()?;

        unsafe {
            let virtual_address =
                crate::boot::limine::physical_to_virtual(pml4.address).expect("HHDM unavailable");

            let table = virtual_address as *mut PageTable;

            (*table).zero();
        }

        crate::drivers::serial::println("PML4 allocated");

        crate::drivers::serial::print("PML4 physical address: 0x");

        crate::drivers::serial::print_u64(pml4.address);

        crate::drivers::serial::println("");

        Some(Self {
            frame_allocator,
            pml4,
        })
    }

    pub fn create_bootstrap_stack(&mut self) -> Option<u64> {
        let page_count = STACK_SIZE / PAGE_SIZE;

        for index in 0..page_count {
            let frame = self.frame_allocator.alloc_frame()?;

            let virtual_address = STACK_BASE + index * PAGE_SIZE;

            if !self.map_page(virtual_address, frame.address, true, false) {
                self.frame_allocator.free_frame(frame);
                return None;
            }
        }

        Some(STACK_BASE + STACK_SIZE)
    }

    pub fn map_page(
        &mut self,
        virtual_address: u64,
        physical_address: u64,
        writable: bool,
        user: bool,
    ) -> bool {
        let (pml4_index, pdpt_index, pd_index, pt_index) =
            crate::arch::x86_64::paging::page_indices(virtual_address);

        let pml4_ptr = match self.table(self.pml4) {
            Some(ptr) => ptr,
            None => return false,
        };

        let pdpt_frame;

        unsafe {
            let pml4 = &mut *pml4_ptr;

            if pml4.entries[pml4_index].is_present() {
                pdpt_frame = PhysFrame {
                    address: pml4.entries[pml4_index].address(),
                };
            } else {
                pdpt_frame = match self.allocate_table() {
                    Some(frame) => frame,
                    None => return false,
                };

                pml4.entries[pml4_index].set(pdpt_frame.address, true, user);
            }
        }

        let pdpt_ptr = match self.table(pdpt_frame) {
            Some(ptr) => ptr,
            None => return false,
        };

        let pd_frame;

        unsafe {
            let pdpt = &mut *pdpt_ptr;

            if pdpt.entries[pdpt_index].is_present() {
                pd_frame = PhysFrame {
                    address: pdpt.entries[pdpt_index].address(),
                };
            } else {
                pd_frame = match self.allocate_table() {
                    Some(frame) => frame,
                    None => return false,
                };

                pdpt.entries[pdpt_index].set(pd_frame.address, true, user);
            }
        }

        let pd_ptr = match self.table(pd_frame) {
            Some(ptr) => ptr,
            None => return false,
        };

        let pt_frame;

        unsafe {
            let pd = &mut *pd_ptr;

            if pd.entries[pd_index].is_present() {
                if pd.entries[pd_index].is_huge() {
                    crate::drivers::serial::println(
                        "ERROR: cannot map 4 KiB page inside huge page",
                    );

                    return false;
                }

                pt_frame = PhysFrame {
                    address: pd.entries[pd_index].address(),
                };
            } else {
                pt_frame = match self.allocate_table() {
                    Some(frame) => frame,
                    None => return false,
                };

                pd.entries[pd_index].set(pt_frame.address, true, user);
            }
        }

        let pt_ptr = match self.table(pt_frame) {
            Some(ptr) => ptr,
            None => return false,
        };

        unsafe {
            let pt = &mut *pt_ptr;

            pt.entries[pt_index].set(physical_address, writable, user);
        }

        true
    }

    fn allocate_table(&mut self) -> Option<PhysFrame> {
        let frame = self.frame_allocator.alloc_frame()?;

        let virtual_address = crate::boot::limine::physical_to_virtual(frame.address)?;

        unsafe {
            let table = virtual_address as *mut PageTable;

            (*table).zero();
        }

        Some(frame)
    }

    fn table(&self, frame: PhysFrame) -> Option<*mut PageTable> {
        let virtual_address = crate::boot::limine::physical_to_virtual(frame.address)?;

        Some(virtual_address as *mut PageTable)
    }

    pub fn map_identity_2mb(&mut self, size: u64) -> bool {
        if size == 0 {
            return false;
        }

        let size = (size + HUGE_PAGE_SIZE - 1) & !(HUGE_PAGE_SIZE - 1);

        let page_count = size / HUGE_PAGE_SIZE;

        if page_count > 512 {
            return false;
        }

        let pdpt = match self.allocate_table() {
            Some(frame) => frame,
            None => return false,
        };

        let pd = match self.allocate_table() {
            Some(frame) => frame,
            None => return false,
        };

        let pml4_ptr = match self.table(self.pml4) {
            Some(ptr) => ptr,
            None => return false,
        };

        let pdpt_ptr = match self.table(pdpt) {
            Some(ptr) => ptr,
            None => return false,
        };

        let pd_ptr = match self.table(pd) {
            Some(ptr) => ptr,
            None => return false,
        };

        unsafe {
            let pml4 = &mut *pml4_ptr;
            let pdpt_table = &mut *pdpt_ptr;
            let pd_table = &mut *pd_ptr;

            pml4.entries[0].set(pdpt.address, true, false);

            pdpt_table.entries[0].set(pd.address, true, false);

            for index in 0..page_count as usize {
                let physical_address = index as u64 * HUGE_PAGE_SIZE;

                pd_table.entries[index].set_huge(physical_address, true, false);
            }
        }

        true
    }

    pub fn pml4(&self) -> PhysFrame {
        self.pml4
    }

    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.frame_allocator.alloc_frame()
    }

    pub fn free_frame(&mut self, frame: PhysFrame) -> bool {
        self.frame_allocator.free_frame(frame)
    }

    pub fn map_hhdm(&mut self) -> bool {
        let response = match crate::boot::limine::HHDM_REQUEST.response() {
            Some(response) => response,
            None => {
                crate::drivers::serial::println("ERROR: HHDM unavailable");
                return false;
            }
        };

        let hhdm_offset = response.offset;

        crate::drivers::serial::print("HHDM offset: 0x");
        crate::drivers::serial::print_u64(hhdm_offset);
        crate::drivers::serial::println("");

        let memory_map = match crate::boot::limine::MEMORY_MAP_REQUEST.response() {
            Some(response) => response,
            None => {
                crate::drivers::serial::println("ERROR: memory map unavailable");
                return false;
            }
        };

        for entry in memory_map.entries() {
            let physical_start = entry.base;
            let length = entry.length;

            if length == 0 {
                continue;
            }

            let page_count = (length + PAGE_SIZE - 1) / PAGE_SIZE;

            for index in 0..page_count {
                let physical_address = physical_start + index * PAGE_SIZE;

                let virtual_address = hhdm_offset + physical_address;

                if !self.map_page(virtual_address, physical_address, true, false) {
                    crate::drivers::serial::println("ERROR: failed to map HHDM page");

                    return false;
                }
            }
        }

        crate::drivers::serial::println("HHDM mapping created");

        true
    }

    pub fn map_kernel(&mut self) -> bool {
        let response = match crate::boot::limine::EXECUTABLE_ADDRESS_REQUEST.response() {
            Some(response) => response,
            None => {
                crate::drivers::serial::println("ERROR: executable address unavailable");
                return false;
            }
        };

        let physical_base = response.physical_base;
        let virtual_base = response.virtual_base;

        let kernel_start = unsafe { &_kernel_start as *const u8 as u64 };

        let kernel_end = unsafe { &_kernel_end as *const u8 as u64 };

        let kernel_size = kernel_end - kernel_start;

        crate::drivers::serial::print("Kernel size: ");
        crate::drivers::serial::print_u64(kernel_size);
        crate::drivers::serial::println(" bytes");

        let page_count = (kernel_size + PAGE_SIZE - 1) / PAGE_SIZE;

        for index in 0..page_count {
            let virtual_address = virtual_base + index * PAGE_SIZE;

            let physical_address = physical_base + index * PAGE_SIZE;

            if !self.map_page(virtual_address, physical_address, true, false) {
                crate::drivers::serial::println("ERROR: failed to map kernel page");

                return false;
            }
        }

        crate::drivers::serial::println("Kernel 4 KiB mapping created");

        true
    }
}

extern "C" fn after_address_space_switch() -> ! {
    crate::drivers::serial::println(
        "Entered new address space!",
    );

    let cr3 = crate::arch::x86_64::paging::read_cr3();

    crate::drivers::serial::print("New CR3: 0x");
    crate::drivers::serial::print_u64(cr3);
    crate::drivers::serial::println("");

    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::println(
        "Interrupts enabled!",
    );

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub fn test() {
    crate::drivers::serial::println("Virtual memory test");

    let frame_allocator = crate::memory::physical::FrameAllocator::new();

    let mut virtual_memory = match VirtualMemoryManager::new(frame_allocator) {
        Some(vm) => vm,

        None => {
            crate::drivers::serial::println("ERROR: failed to create virtual memory manager");

            return;
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

            return;
        }
    };

    crate::drivers::serial::println("Entering new address space...");

    let entry = after_address_space_switch as usize as u64;

    unsafe {
        enter_new_address_space(pml4.address, stack_top, entry);
    }
}
