use crate::arch::x86_64::paging::PageTable;
use crate::memory::physical::{FrameAllocator, PhysFrame};

const HUGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;

unsafe extern "C" {
    static _kernel_start: u8;
    static _kernel_end: u8;
}

pub struct VirtualMemoryManager {
    frame_allocator: FrameAllocator,
    pml4: PhysFrame,
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

    fn map_kernel(&mut self) -> bool {
        let response = match crate::boot::limine::EXECUTABLE_ADDRESS_REQUEST.response() {
            Some(response) => response,
            None => {
                crate::drivers::serial::println("ERROR: executable address unvaible");
                return false;
            }
        };

        let physical_base = response.physical_base;
        let virtual_base = response.virtual_base;

        let kernel_start = unsafe { &_kernel_start as *const u8 as u64 };
        let kernel_end = unsafe { &_kernel_end as *const u8 as u64 };
        let kernel_size = kernel_end - kernel_start;

        crate::drivers::serial::print("Kernel physical base: 0x");
        crate::drivers::serial::print_u64(physical_base);
        crate::drivers::serial::println("");

        crate::drivers::serial::print("Kernel virtual base: 0x");
        crate::drivers::serial::print_u64(virtual_base);
        crate::drivers::serial::println("");

        crate::drivers::serial::print("Kernel size: ");
        crate::drivers::serial::print_u64(kernel_size);
        crate::drivers::serial::println(" bytes");

        let mapping_start = virtual_base & !(HUGE_PAGE_SIZE - 1);
        let mapping_end = (virtual_base + kernel_size + HUGE_PAGE_SIZE - 1) & !(HUGE_PAGE_SIZE - 1);
        let mapping_size = mapping_end - mapping_start;

        let page_count = mapping_size / HUGE_PAGE_SIZE;
        if page_count == 0 {
            return false;
        }

        if page_count > 512 {
            crate::drivers::serial::println("ERROR: kernel is larger than 1 GiB");
            return false;
        }

        let pml4_index = ((mapping_start >> 39) & 0x1FF) as usize;

        let pdpt_index = ((mapping_start >> 30) & 0x1FF) as usize;

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

            pml4.entries[pml4_index].set(pdpt.address, true, false);

            pdpt_table.entries[pdpt_index].set(pd.address, true, false);

            let physical_start = physical_base - (virtual_base - mapping_start);

            for index in 0..page_count as usize {
                let physical = physical_start + index as u64 * HUGE_PAGE_SIZE;

                pd_table.entries[index].set_huge(physical, true, false);
            }
        }

        true
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

    crate::drivers::serial::println("Testing kernel address...");

    let response = crate::boot::limine::EXECUTABLE_ADDRESS_REQUEST.response();

    match response {
        Some(response) => {
            crate::drivers::serial::print("Executable physical base: 0x");

            crate::drivers::serial::print_u64(response.physical_base);

            crate::drivers::serial::println("");

            crate::drivers::serial::print("Executable virtual base: 0x");

            crate::drivers::serial::print_u64(response.virtual_base);

            crate::drivers::serial::println("");
        }

        None => {
            crate::drivers::serial::println("ERROR: executable address unavailable");
        }
    }
}
