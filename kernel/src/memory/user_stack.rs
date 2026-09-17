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
}
