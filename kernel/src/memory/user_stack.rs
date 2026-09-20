use crate::boot::limine::physical_to_virtual;
use crate::memory::address_space::AddressSpace;
use crate::memory::physical::FrameAllocator;
use crate::arch::x86_64::paging::PAGE_SIZE;

const USER_MAX: u64 = 0x0000_8000_0000_0000;

pub const USER_STACK_TOP: u64 = USER_MAX;

pub const USER_STACK_PAGES: usize = 4;

pub struct UserStack {
    pub base: u64,
    pub top: u64,
    pub size: u64,
}

fn write_bytes_to_user(address_space: &AddressSpace, mut va: u64, data: &[u8]) -> bool {
    for &b in data {
        let page_base = va & !0xFFF;
        let offset = va & 0xFFF;
        let phys_base = match address_space.translate_page(page_base) {
            Some(p) => p,
            None => return false,
        };
        let phys = phys_base + offset;
        let hva = match physical_to_virtual(phys) {
            Some(v) => v,
            None => return false,
        };
        unsafe {
            core::ptr::write_volatile(hva as *mut u8, b);
        }
        va = match va.checked_add(1) {
            Some(v) => v,
            None => return false,
        };
    }
    true
}

fn write_u64_to_user(address_space: &AddressSpace, va: u64, value: u64) -> bool {
    write_bytes_to_user(address_space, va, &value.to_le_bytes())
}

impl UserStack {
    pub fn map(
        address_space: &mut AddressSpace,
        frame_allocator: &mut FrameAllocator,
    ) -> Option<Self> {
        let size = USER_STACK_PAGES as u64 * PAGE_SIZE;

        let top = USER_STACK_TOP;
        let base = top.checked_sub(size)?;

        for i in 0..USER_STACK_PAGES {
            let virtual_address = base + (i as u64 * PAGE_SIZE);

            let frame = match frame_allocator.alloc_frame() {
                Some(frame) => frame,
                None => {
                    Self::cleanup(address_space, frame_allocator, base, i);
                    return None;
                }
            };

            if !address_space.map_page(frame_allocator, virtual_address, frame.address, true, false)
            {
                frame_allocator.free_frame(frame);

                Self::cleanup(address_space, frame_allocator, base, i);

                return None;
            }
        }

        crate::drivers::serial::println("User stack: mapped");

        Some(Self { base, top, size })
    }

    fn cleanup(
        address_space: &mut AddressSpace,
        frame_allocator: &mut FrameAllocator,
        base: u64,
        pages: usize,
    ) {
        for i in 0..pages {
            let virtual_address = base + (i as u64 * PAGE_SIZE);

            let _ = address_space.unmap_page(frame_allocator, virtual_address);
        }
    }

    pub fn contains(&self, virtual_address: u64) -> bool {
        virtual_address >= self.base && virtual_address < self.top
    }

    pub fn setup_argv(
        &self,
        address_space: &AddressSpace,
        argv: &[&str],
    ) -> Option<u64> {
        if self.base >= USER_MAX || self.top > USER_MAX || self.base >= self.top {
            return None;
        }
        if self.size != USER_STACK_PAGES as u64 * PAGE_SIZE {
            
        }

        let n = argv.len() as u64;

        let mut strings_total: u64 = 0;
        for s in argv {
            let len = s.len() as u64;
            let need = len.checked_add(1)?;
            strings_total = strings_total.checked_add(need)?;
            if len > 4096 * 4 {
                return None;
            }
        }

        let pointers_total = n.checked_add(1)?.checked_mul(8)?;
        let argc_size: u64 = 8;

        let minimal = strings_total.checked_add(pointers_total)?.checked_add(argc_size)?;
        if minimal > self.size {
            return None;
        }
        if minimal.checked_add(22).is_none() {
            return None;
        }
        if minimal + 22 > self.size && minimal != self.size {
        }

        let s0 = self.top.checked_sub(strings_total)?;
        if s0 < self.base {
            return None;
        }
        if s0 >= USER_MAX {
            return None;
        }
        
        let s1 = s0 & !7u64;
        if s1 < self.base {
            return None;
        }
        let r0 = s1.checked_sub(pointers_total)?.checked_sub(argc_size)?;
        if r0 < self.base {
            return None;
        }
        let rsp = r0 & !15u64;
        if rsp < self.base {
            return None;
        }
        if rsp >= USER_MAX {
            return None;
        }
        if rsp & 0xF != 0 {
            return None;
        }
        let argv_start = rsp.checked_add(argc_size)?;
        let argv_end = argv_start.checked_add(pointers_total)?;
        if argv_end > s0 {
            return None;
        }
        if s0.checked_add(strings_total)? != self.top {
            return None;
        }

        let mut cur_str_va = s0;
        let mut str_addrs: alloc::vec::Vec<u64> = alloc::vec::Vec::new();

        if str_addrs.try_reserve(n as usize).is_err() {
            return None;
        }

        for s in argv {
            str_addrs.push(cur_str_va);
            if !write_bytes_to_user(address_space, cur_str_va, s.as_bytes()) {
                return None;
            }
            let nul_va = cur_str_va.checked_add(s.len() as u64)?;
            if !write_bytes_to_user(address_space, nul_va, &[0]) {
                return None;
            }
            cur_str_va = cur_str_va.checked_add(s.len() as u64)?.checked_add(1)?;
        }
        if cur_str_va != self.top {
            return None;
        }

        if !write_u64_to_user(address_space, rsp, n) {
            return None;
        }

        for (i, &addr) in str_addrs.iter().enumerate() {
            let ptr_va = argv_start.checked_add((i as u64) * 8)?;
            if !write_u64_to_user(address_space, ptr_va, addr) {
                return None;
            }
        }
        let null_ptr_va = argv_start.checked_add(n * 8)?;
        if !write_u64_to_user(address_space, null_ptr_va, 0) {
            return None;
        }

        let mut pad_va = argv_end;
        while pad_va < s0 {
            if !write_bytes_to_user(address_space, pad_va, &[0]) {
                return None;
            }
            pad_va = pad_va.checked_add(1)?;
            if pad_va - argv_end > 32 {
                break;
            }
        }
        let pad2 = rsp.checked_add(argc_size)?.checked_add(pointers_total)?;
        let _ = pad2;

        Some(rsp)
    }
}
