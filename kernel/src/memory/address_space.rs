use crate::arch::x86_64::paging::PAGE_SIZE;
use crate::arch::x86_64::paging::PageTable;
use crate::boot::limine::physical_to_virtual;
use crate::memory::physical::{FrameAllocator, PhysFrame};

const KERNEL_PML4_START: usize = 256;

pub struct AddressSpace {
    pml4: PhysFrame,
}

impl AddressSpace {
    pub fn new(frame_allocator: &mut FrameAllocator) -> Option<Self> {
        let pml4 = frame_allocator.alloc_frame()?;

        let virtual_address = physical_to_virtual(pml4.address)?;

        unsafe {
            let table = virtual_address as *mut PageTable;
            (*table).zero();
        }

        crate::drivers::serial::print("AddressSpace: new PML4 = 0x");
        crate::drivers::serial::print_u64(pml4.address);
        crate::drivers::serial::println("");

        Some(Self { pml4 })
    }

    pub fn new_user(frame_allocator: &mut FrameAllocator) -> Option<Self> {
        let pml4 = frame_allocator.alloc_frame()?;

        let new_pml4_va = physical_to_virtual(pml4.address)?;

        let kernel_pml4_addr = match crate::memory::virtualm::try_kernel_pml4() {
            Some(frame) => frame.address,
            None => crate::arch::x86_64::paging::read_cr3() & !0xFFF,
        };

        let kernel_pml4_va = physical_to_virtual(kernel_pml4_addr)?;

        unsafe {
            let new_table = new_pml4_va as *mut PageTable;
            let kernel_table = kernel_pml4_va as *const PageTable;

            (*new_table).zero();

            for index in KERNEL_PML4_START..512 {
                (*new_table).entries[index] = (*kernel_table).entries[index];
            }
        }

        crate::drivers::serial::print("AddressSpace: user PML4 = 0x");
        crate::drivers::serial::print_u64(pml4.address);
        crate::drivers::serial::println("");

        crate::drivers::serial::println("AddressSpace: kernel mappings copied");

        Some(Self { pml4 })
    }

    pub fn pml4(&self) -> PhysFrame {
        self.pml4
    }

    pub fn map_page(
        &mut self,
        frame_allocator: &mut FrameAllocator,
        virtual_address: u64,
        physical_address: u64,
        writable: bool,
        executable: bool,
    ) -> bool {
        const USER_MAX: u64 = 0x0000_8000_0000_0000;

        if virtual_address % 4096 != 0 {
            return false;
        }

        if physical_address % 4096 != 0 {
            return false;
        }

        if virtual_address >= USER_MAX {
            return false;
        }

        let (pml4_index, pdpt_index, pd_index, pt_index) =
            crate::arch::x86_64::paging::page_indices(virtual_address);

        let pml4_va = match physical_to_virtual(self.pml4.address) {
            Some(va) => va,
            None => return false,
        };

        unsafe {
            let pml4 = &mut *(pml4_va as *mut PageTable);

            let pdpt_frame;

            if !pml4.entries[pml4_index].is_present() {
                let frame = match frame_allocator.alloc_frame() {
                    Some(frame) => frame,
                    None => return false,
                };

                let table_va = match physical_to_virtual(frame.address) {
                    Some(va) => va,
                    None => return false,
                };

                (*(table_va as *mut PageTable)).zero();

                pml4.entries[pml4_index].set(frame.address, true, true, true);

                pdpt_frame = frame;
            } else {
                pdpt_frame = PhysFrame::new(pml4.entries[pml4_index].address());
            }

            let pdpt_va = match physical_to_virtual(pdpt_frame.address) {
                Some(va) => va,
                None => return false,
            };

            let pdpt = &mut *(pdpt_va as *mut PageTable);

            let pd_frame;

            if !pdpt.entries[pdpt_index].is_present() {
                let frame = match frame_allocator.alloc_frame() {
                    Some(frame) => frame,
                    None => return false,
                };

                let table_va = match physical_to_virtual(frame.address) {
                    Some(va) => va,
                    None => return false,
                };

                (*(table_va as *mut PageTable)).zero();

                pdpt.entries[pdpt_index].set(frame.address, true, true, true);

                pd_frame = frame;
            } else {
                if pdpt.entries[pdpt_index].is_huge() {
                    return false;
                }

                pd_frame = PhysFrame::new(pdpt.entries[pdpt_index].address());
            }

            let pd_va = match physical_to_virtual(pd_frame.address) {
                Some(va) => va,
                None => return false,
            };

            let pd = &mut *(pd_va as *mut PageTable);

            let pt_frame;

            if !pd.entries[pd_index].is_present() {
                let frame = match frame_allocator.alloc_frame() {
                    Some(frame) => frame,
                    None => return false,
                };

                let table_va = match physical_to_virtual(frame.address) {
                    Some(va) => va,
                    None => return false,
                };

                (*(table_va as *mut PageTable)).zero();

                pd.entries[pd_index].set(frame.address, true, true, true);

                pt_frame = frame;
            } else {
                if pd.entries[pd_index].is_huge() {
                    return false;
                }

                pt_frame = PhysFrame::new(pd.entries[pd_index].address());
            }

            let pt_va = match physical_to_virtual(pt_frame.address) {
                Some(va) => va,
                None => return false,
            };

            let pt = &mut *(pt_va as *mut PageTable);

            if pt.entries[pt_index].is_present() {
                return false;
            }

            pt.entries[pt_index].set(physical_address, writable, true,  executable);
        }

        true
    }

    pub fn map_user_page(
        &mut self,
        frame_allocator: &mut FrameAllocator,
        virtual_address: u64,
        physical_address: u64,
        writable: bool,
    ) -> bool {
        if virtual_address % PAGE_SIZE != 0 {
            return false;
        }

        if physical_address % PAGE_SIZE != 0 {
            return false;
        }

        if virtual_address >= 0x0000_8000_0000_0000 {
            return false;
        }

        self.map_page(frame_allocator, virtual_address, physical_address, writable, false)
    }

    pub fn translate_page(&self, virtual_address: u64) -> Option<u64> {
        if virtual_address % 4096 != 0 {
            return None;
        }

        const USER_MAX: u64 = 0x0000_8000_0000_0000;

        if virtual_address >= USER_MAX {
            return None;
        }

        let (pml4_index, pdpt_index, pd_index, pt_index) =
            crate::arch::x86_64::paging::page_indices(virtual_address);

        let pml4_va = physical_to_virtual(self.pml4.address)?;

        unsafe {
            let pml4 = &*(pml4_va as *const PageTable);

            if !pml4.entries[pml4_index].is_present() {
                return None;
            }

            let pdpt_frame = PhysFrame::new(pml4.entries[pml4_index].address());
            let pdpt_va = physical_to_virtual(pdpt_frame.address)?;
            let pdpt = &*(pdpt_va as *const PageTable);

            if !pdpt.entries[pdpt_index].is_present() {
                return None;
            }

            if pdpt.entries[pdpt_index].is_huge() {
                return None;
            }

            let pd_frame = PhysFrame::new(pdpt.entries[pdpt_index].address());
            let pd_va = physical_to_virtual(pd_frame.address)?;
            let pd = &*(pd_va as *const PageTable);

            if !pd.entries[pd_index].is_present() {
                return None;
            }

            if pd.entries[pd_index].is_huge() {
                return None;
            }

            let pt_frame = PhysFrame::new(pd.entries[pd_index].address());
            let pt_va = physical_to_virtual(pt_frame.address)?;
            let pt = &*(pt_va as *const PageTable);

            if !pt.entries[pt_index].is_present() {
                return None;
            }

            Some(pt.entries[pt_index].address())
        }
    }
    pub fn unmap_page(
        &mut self,
        frame_allocator: &mut FrameAllocator,
        virtual_address: u64,
    ) -> Option<PhysFrame> {
        if virtual_address % 4096 != 0 {
            return None;
        }

        const USER_MAX: u64 = 0x0000_8000_0000_0000;

        if virtual_address >= USER_MAX {
            return None;
        }

        let (pml4_index, pdpt_index, pd_index, pt_index) =
            crate::arch::x86_64::paging::page_indices(virtual_address);
        let pml4_va = physical_to_virtual(self.pml4.address)?;

        unsafe {
            let pml4 = &mut *(pml4_va as *mut PageTable);

            if !pml4.entries[pml4_index].is_present() {
                return None;
            }

            let pdpt_frame = PhysFrame::new(pml4.entries[pml4_index].address());
            let pdpt_va = physical_to_virtual(pdpt_frame.address)?;
            let pdpt = &mut *(pdpt_va as *mut PageTable);

            if !pdpt.entries[pdpt_index].is_present() || pdpt.entries[pdpt_index].is_huge() {
                return None;
            }

            let pd_frame = PhysFrame::new(pdpt.entries[pdpt_index].address());
            let pd_va = physical_to_virtual(pd_frame.address)?;
            let pd = &mut *(pd_va as *mut PageTable);

            if !pd.entries[pd_index].is_present() || pd.entries[pd_index].is_huge() {
                return None;
            }

            let pt_frame = PhysFrame::new(pd.entries[pd_index].address());
            let pt_va = physical_to_virtual(pt_frame.address)?;
            let pt = &mut *(pt_va as *mut PageTable);

            if !pt.entries[pt_index].is_present() {
                return None;
            }

            let physical_address = pt.entries[pt_index].address();

            pt.entries[pt_index].clear();

            let frame = PhysFrame::new(physical_address);

            frame_allocator.free_frame(frame);

            Some(frame)
        }
    }
}
