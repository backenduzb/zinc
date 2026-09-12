use crate::{boot::limine::MEMORY_MAP_REQUEST, drivers::display::framebuffer, memory};

const PAGE_SIZE: u64 = 4096;

#[derive(Clone, Copy)]
pub struct PhysFrame {
    pub address: u64,
}

impl PhysFrame {
    pub const fn new(address: u64) -> Self {
        Self { address }
    }
}

pub struct FrameAllocator {
    current_address: u64,
    region_end: u64,
}

impl FrameAllocator {
    pub const fn empty() -> Self {
        Self {
            current_address: 0,
            region_end: 0,
        }
    }

    pub fn new() -> Self {
        let Some(response) = MEMORY_MAP_REQUEST.response() else {
            crate::drivers::serial::println("ERROR: memory map unvaible");
            return Self::empty();
        };

        for entry in response.entries() {
            if entry.type_ == limine::memmap::MEMMAP_USABLE {
                let start = align_up(entry.base);
                let end = entry.base + entry.length;

                if start < end {
                    return Self {
                        current_address: start,
                        region_end: end,
                    };
                }
            }
        }

        crate::drivers::serial::println("ERROR: no usable memory region found");
        Self::empty()
    }

    pub fn alloc_frame(&mut self) -> Option<PhysFrame> {
        if self.current_address >= self.region_end {
            return None;
        }

        let frame = PhysFrame::new(self.current_address);
        self.current_address += PAGE_SIZE;
        Some(frame)
    }
}

pub fn init() {
    let Some(response) = MEMORY_MAP_REQUEST.response() else {
        crate::drivers::serial::println("ERROR: memory map unvaible");
        return;
    };

    let entries = response.entries();

    crate::drivers::serial::print("Memory map entries: ");
    crate::drivers::serial::print_u64(entries.len() as u64);
    crate::drivers::serial::println("");

    let mut usable_memory = 0u64;

    for entry in entries {
        if entry.type_ == limine::memmap::MEMMAP_USABLE {
            usable_memory += entry.length;
        }
    }

    crate::drivers::serial::print("Usable memory: ");
    crate::drivers::serial::print_u64(usable_memory / (1024 * 1024));
    crate::drivers::serial::println("Mib");

    crate::drivers::serial::println("Physical memory manager initalized");
}

pub fn align_up(address: u64) -> u64 {
    (address + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

pub fn test() {
    let mut frame_allocator = memory::physical::FrameAllocator::new();

    crate::drivers::serial::println("Frame allocator test");

    for _ in  0..5 {
        match frame_allocator.alloc_frame() {
            Some(frame) => {
                crate::drivers::serial::print("Allocated frame: 0x");
                crate::drivers::serial::print_u64(frame.address);
                crate::drivers::serial::println("");
            }

            None => {
                crate::drivers::serial::println(
                    "ERROR: out of physical memory"
                );
                break;
            }
        }
    }
}