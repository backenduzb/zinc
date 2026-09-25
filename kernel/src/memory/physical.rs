use core::sync::atomic::{AtomicBool, Ordering};

use crate::{boot::limine::MEMORY_MAP_REQUEST, memory};

static ALLOCATOR_INITIALIZED: AtomicBool = AtomicBool::new(false);

const PAGE_SIZE: u64 = 4096;

const MAX_PHYSICAL_MEMORY: u64 = 4 * 1024 * 1024 * 1024;
const MAX_FRAMES: usize = (MAX_PHYSICAL_MEMORY / PAGE_SIZE) as usize;

const BITMAP_SIZE: usize = (MAX_FRAMES + 63) / 64;
static mut FRAME_BITMAP: [u64; BITMAP_SIZE] = [0; BITMAP_SIZE];

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
    initialized: bool,
}

impl FrameAllocator {
    pub const fn empty() -> Self {
        Self { initialized: false }
    }

    pub fn new() -> Self {
        if ALLOCATOR_INITIALIZED.swap(true, Ordering::SeqCst) {
            return Self { initialized: true };
        }

        let Some(response) = MEMORY_MAP_REQUEST.response() else {
            crate::log_boot!("ERROR: memory map unavailable");

            ALLOCATOR_INITIALIZED.store(false, Ordering::SeqCst);
            return Self::empty();
        };

        let mut usable_memory = 0u64;

        unsafe {
            let bitmap_ptr = core::ptr::addr_of_mut!(FRAME_BITMAP);

            for i in 0..BITMAP_SIZE {
                (*bitmap_ptr)[i] = u64::MAX;
            }

            for entry in response.entries() {
                if entry.type_ != limine::memmap::MEMMAP_USABLE {
                    continue;
                }

                let region_start = align_up(entry.base);
                let region_end = align_down(entry.base + entry.length);

                if region_start >= region_end {
                    continue;
                }

                let mut address = region_start;

                while address < region_end {
                    if address < MAX_PHYSICAL_MEMORY {
                        let frame_index = (address / PAGE_SIZE) as usize;

                        if frame_index < MAX_FRAMES {
                            free_bitmap_frame(frame_index);
                            usable_memory += PAGE_SIZE;
                        }
                    }

                    address += PAGE_SIZE;
                }
            }
        }

        crate::log_boot_raw!("PMM usable frames: ");
        crate::log_boot_raw!("{}", usable_memory / PAGE_SIZE);
        crate::log_boot!("");
        crate::log_boot!("Frame allocator initialized");

        Self { initialized: true }
    }

    pub fn alloc_frame(&mut self) -> Option<PhysFrame> {
        if !self.initialized {
            return None;
        }

        unsafe {
            for word_index in 0..BITMAP_SIZE {
                let word = FRAME_BITMAP[word_index];

                if word == u64::MAX {
                    continue;
                }

                let free_bit = (!word).trailing_zeros();
                let frame_index = word_index * 64 + free_bit as usize;

                if frame_index >= MAX_FRAMES {
                    return None;
                }

                FRAME_BITMAP[word_index] |= 1u64 << free_bit;
                let address = (frame_index as u64) * PAGE_SIZE;
                return Some(PhysFrame::new(address));
            }
        }

        None
    }

    pub fn free_frame(&mut self, frame: PhysFrame) -> bool {
        if !self.initialized {
            return false;
        }

        let address = frame.address;

        if address % PAGE_SIZE != 0 {
            return false;
        }

        if address >= MAX_PHYSICAL_MEMORY {
            return false;
        }
        let frame_index = (address / PAGE_SIZE) as usize;
        if frame_index >= MAX_FRAMES {
            return false;
        }

        unsafe {
            let word_index = frame_index / 64;
            let bit_index = frame_index % 64;

            let mask = 1u64 << bit_index;

            if FRAME_BITMAP[word_index] & mask == 0 {
                return false;
            }

            FRAME_BITMAP[word_index] &= !mask;
        }

        true
    }

    pub fn free_count(&self) -> usize {
        if !self.initialized {
            return 0;
        }
        unsafe {
            let mut count = 0usize;
            for i in 0..BITMAP_SIZE {
                let word = FRAME_BITMAP[i];
                count += (!word).count_ones() as usize;
            }
            count
        }
    }

    pub fn count_free_frames(&self) -> usize {
        self.free_count()
    }

    pub fn free_frames(&self) -> usize {
        self.free_count()
    }
}

pub fn init() {
    let Some(response) = MEMORY_MAP_REQUEST.response() else {
        crate::log_boot!("ERROR: memory map unvaible");
        return;
    };

    let entries = response.entries();

    crate::log_boot_raw!("Memory map entries: ");
    crate::log_boot_raw!("{}", entries.len() as u64);
    crate::log_boot!("");

    let mut usable_memory = 0u64;

    for entry in entries {
        if entry.type_ == limine::memmap::MEMMAP_USABLE {
            usable_memory += entry.length;
        }
    }

    crate::log_boot_raw!("Usable memory: ");
    crate::log_boot_raw!("{}", usable_memory / (1024 * 1024));
    crate::log_boot!("Mib");

    crate::log_boot!("Physical memory manager initialized");
}

pub fn align_up(address: u64) -> u64 {
    (address + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

pub fn align_down(address: u64) -> u64 {
    address & !(PAGE_SIZE - 1)
}
pub fn free_bitmap_frame(frame_index: usize) {
    let word_index = frame_index / 64;
    let bit_index = frame_index % 64;

    unsafe {
        FRAME_BITMAP[word_index] &= !(1u64 << bit_index);
    }
}

pub fn test() {
    let mut frame_allocator = memory::physical::FrameAllocator::new();

    crate::log_test!("Frame allocator test");

    let frame1 = frame_allocator.alloc_frame();
    let frame2 = frame_allocator.alloc_frame();
    let frame3 = frame_allocator.alloc_frame();

    if let Some(frame) = frame1 {
        crate::log_boot_raw!("Allocated frame 1: 0x");

        crate::log_boot_raw!("{}", frame.address);

        crate::log_boot!("");
    }

    if let Some(frame) = frame2 {
        crate::log_boot_raw!("Allocated frame 2: 0x");

        crate::log_boot_raw!("{}", frame.address);

        crate::log_boot!("");
    }

    if let Some(frame) = frame3 {
        crate::log_boot_raw!("Allocated frame 3: 0x");

        crate::log_boot_raw!("{}", frame.address);

        crate::log_boot!("");
    }

    if let Some(frame) = frame2 {
        if frame_allocator.free_frame(frame) {
            crate::log_boot!("Frame 2 freed");
        } else {
            crate::log_test!("ERROR: failed to free frame 2");
        }
    }

    match frame_allocator.alloc_frame() {
        Some(frame) => {
            crate::log_boot_raw!("Reallocated frame: 0x");

            crate::log_boot_raw!("{}", frame.address);

            crate::log_boot!("");
        }

        None => {
            crate::log_test!("ERROR: reallocation failed");
        }
    }
}
