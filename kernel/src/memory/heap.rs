use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const HEAP_START: usize = 0xFFFF_FFFF_9000_0000;
const HEAP_SIZE: usize = 1024 * 1024;

pub struct KernelHeap {
    start: usize,
    size: usize,
    offset: AtomicUsize,
    initialized: AtomicBool,
}

impl KernelHeap {
    pub const fn new() -> Self {
        Self {
            start: HEAP_START,
            size: HEAP_SIZE,
            offset: AtomicUsize::new(0),
            initialized: AtomicBool::new(false),
        }
    }

    pub fn init(&self) {
        self.offset.store(0, Ordering::Relaxed);
        self.initialized.store(true, Ordering::Release);
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::Acquire)
    }

    pub fn used(&self) -> usize {
        self.offset.load(Ordering::Relaxed)
    }

    pub fn remaining(&self) -> usize {
        self.size.saturating_sub(self.used())
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !self.initialized.load(Ordering::Acquire) {
            return null_mut();
        }

        let size = layout.size();
        let align = layout.align();

        if size == 0 {
            return null_mut();
        }

        let mut current = self.offset.load(Ordering::Relaxed);

        loop {
            let aligned = match current.checked_add(align - 1) {
                Some(value) => value & !(align - 1),
                None => return null_mut(),
            };

            let new_offset = match aligned.checked_add(size) {
                Some(value) => value,
                None => return null_mut(),
            };

            if new_offset > self.size {
                return null_mut();
            }

            match self.offset.compare_exchange_weak(
                current,
                new_offset,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return (self.start + aligned) as *mut u8;
                }
                Err(actual) => {
                    current = actual;
                }
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
pub static KERNEL_HEAP: KernelHeap = KernelHeap::new();