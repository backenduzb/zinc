use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, Ordering};

pub const HEAP_START: usize = 0xFFFF_FFFF_9000_0000;
pub const HEAP_SIZE: usize = 8 * 1024 * 1024; 

struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

struct HeapInner {
    start: usize,
    end: usize,
    free_head: *mut FreeBlock,
    initialized: bool,
}

impl HeapInner {
    const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            free_head: null_mut(),
            initialized: false,
        }
    }
}

static mut HEAP_INNER: HeapInner = HeapInner::new();
static HEAP_LOCK: AtomicBool = AtomicBool::new(false);
static HEAP_INIT_FLAG: AtomicBool = AtomicBool::new(false);

fn acquire_lock() {
    while HEAP_LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
}

fn release_lock() {
    HEAP_LOCK.store(false, Ordering::Release);
}

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

pub struct KernelHeap;

impl KernelHeap {
    pub const fn new() -> Self {
        Self
    }

    pub fn init(&self) {
        acquire_lock();
        unsafe {
            let inner = &raw mut HEAP_INNER;
            if (*inner).initialized {
                release_lock();
                return;
            }
            (*inner).start = HEAP_START;
            (*inner).end = HEAP_START + HEAP_SIZE;
            let head = HEAP_START as *mut FreeBlock;
            (*head).size = HEAP_SIZE;
            (*head).next = null_mut();
            (*inner).free_head = head;
            (*inner).initialized = true;
        }
        HEAP_INIT_FLAG.store(true, Ordering::Release);
        release_lock();
    }

    pub fn is_initialized(&self) -> bool {
        HEAP_INIT_FLAG.load(Ordering::Acquire)
    }

    pub fn used(&self) -> usize {
        acquire_lock();
        let used = unsafe {
            if !(&raw const HEAP_INNER).as_ref().unwrap().initialized {
                0
            } else {
                let mut free = 0usize;
                let mut cur = (&raw const HEAP_INNER).as_ref().unwrap().free_head;
                while !cur.is_null() {
                    free += (*cur).size;
                    cur = (*cur).next;
                }
                HEAP_SIZE - free
            }
        };
        release_lock();
        used
    }

    pub fn remaining(&self) -> usize {
        HEAP_SIZE.saturating_sub(self.used())
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        acquire_lock();
        let out = unsafe {
            if !(&raw const HEAP_INNER).as_ref().unwrap().initialized {
                (0, 0, 0)
            } else {
                let mut free = 0usize;
                let mut largest = 0usize;
                let mut count = 0usize;
                let mut cur = (&raw const HEAP_INNER).as_ref().unwrap().free_head;
                let mut guard = 0usize;
                while !cur.is_null() && guard < 100000 {
                    let sz = (*cur).size;
                    free = free.saturating_add(sz);
                    if sz > largest {
                        largest = sz;
                    }
                    count += 1;
                    cur = (*cur).next;
                    guard += 1;
                }
                (free, largest, count)
            }
        };
        release_lock();
        out
    }

    pub fn log_state(&self, tag: &str) {
        let (free_total, largest, blocks) = self.stats();
        let used = HEAP_SIZE.saturating_sub(free_total);
        crate::drivers::serial::print("HEAP[");
        crate::drivers::serial::print(tag);
        crate::drivers::serial::print("] used=");
        crate::drivers::serial::print_u64(used as u64);
        crate::drivers::serial::print(" free=");
        crate::drivers::serial::print_u64(free_total as u64);
        crate::drivers::serial::print(" largest=");
        crate::drivers::serial::print_u64(largest as u64);
        crate::drivers::serial::print(" blocks=");
        crate::drivers::serial::print_u64(blocks as u64);
        crate::drivers::serial::println("");
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return null_mut();
        }
        if !HEAP_INIT_FLAG.load(Ordering::Acquire) {
            return null_mut();
        }
        let raw_size = layout.size();
        let align = layout.align().max(core::mem::align_of::<FreeBlock>());
        let size = raw_size.max(core::mem::size_of::<FreeBlock>());

        acquire_lock();
        let mut result = null_mut();
        let mut found = false;
        unsafe {
            let inner = &raw mut HEAP_INNER;
            if !(*inner).initialized {
                release_lock();
                return null_mut();
            }
            let mut prev: *mut *mut FreeBlock = &raw mut (*inner).free_head;
            let mut cur = (*inner).free_head;
            while !cur.is_null() {
                let block_start = cur as usize;
                let block_size = (*cur).size;
                let block_end = block_start + block_size;
                let aligned_start = align_up(block_start, align);
                let aligned_end = match aligned_start.checked_add(size) {
                    Some(v) => v,
                    None => {
                        prev = &mut (*cur).next as *mut *mut FreeBlock;
                        cur = (*cur).next;
                        continue;
                    }
                };
                if aligned_end > block_end {
                    prev = &mut (*cur).next as *mut *mut FreeBlock;
                    cur = (*cur).next;
                    continue;
                }
                let next = (*cur).next;
                *prev = next;

                if aligned_start > block_start {
                    let prefix_size = aligned_start - block_start;
                    if prefix_size >= core::mem::size_of::<FreeBlock>() {
                        let prefix = block_start as *mut FreeBlock;
                        (*prefix).size = prefix_size;
                        (*prefix).next = *prev;
                        *prev = prefix;
                        prev = &mut (*prefix).next as *mut *mut FreeBlock;
                    } else if prefix_size > 0 {
                    }
                }
                
                let suffix_size = block_end - aligned_end;
                if suffix_size >= core::mem::size_of::<FreeBlock>() {
                    let suffix = aligned_end as *mut FreeBlock;
                    (*suffix).size = suffix_size;
                    (*suffix).next = *prev;
                    *prev = suffix;
                } else if suffix_size > 0 {
                }
                result = aligned_start as *mut u8;
                found = true;
                break;
            }
        }
        release_lock();
        if !found {
            crate::drivers::serial::print("HEAP ALLOC FAIL size=");
            crate::drivers::serial::print_u64(size as u64);
            crate::drivers::serial::print(" align=");
            crate::drivers::serial::print_u64(align as u64);
            crate::drivers::serial::println("");
            KERNEL_HEAP.log_state("alloc-fail");
            return null_mut();
        }
        result
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() || layout.size() == 0 {
            return;
        }
        if !HEAP_INIT_FLAG.load(Ordering::Acquire) {
            return;
        }
        let size = layout.size();
        acquire_lock();
        unsafe {
            let inner = &raw mut HEAP_INNER;
            if !(*inner).initialized {
                release_lock();
                return;
            }
            let block_start = ptr as usize;
            let block_size = {
                let min = core::mem::size_of::<FreeBlock>();
                if size < min { min } else { size }
            };
            let mut prev: *mut *mut FreeBlock = &raw mut (*inner).free_head;
            let mut cur = (*inner).free_head;
            while !cur.is_null() && (cur as usize) < block_start {
                prev = &mut (*cur).next as *mut *mut FreeBlock;
                cur = (*cur).next;
            }
            let new_block = block_start as *mut FreeBlock;
            (*new_block).size = block_size;
            (*new_block).next = cur;
            *prev = new_block;

            if !cur.is_null() {
                let new_end = block_start + block_size;
                let next_start = cur as usize;
                if new_end == next_start {
                    (*new_block).size += (*cur).size;
                    (*new_block).next = (*cur).next;
                } else if new_end > next_start {
                    let overlap_end = (cur as usize) + (*cur).size;
                    let merged_end = core::cmp::max(new_end, overlap_end);
                    (*new_block).size = merged_end - block_start;
                    (*new_block).next = (*cur).next;
                }
            }

            let mut scan_prev: *mut FreeBlock = null_mut();
            let mut scan = (*inner).free_head;
            while !scan.is_null() && scan != new_block {
                scan_prev = scan;
                scan = (*scan).next;
            }
            if !scan_prev.is_null() {
                let prev_start = scan_prev as usize;
                let prev_end = prev_start + (*scan_prev).size;
                if prev_end == block_start {
                    (*scan_prev).size += (*new_block).size;
                    (*scan_prev).next = (*new_block).next;
                } else if prev_end > block_start {
                    let new_end = block_start + (*new_block).size;
                    let merged_end = core::cmp::max(prev_end, new_end);
                    (*scan_prev).size = merged_end - prev_start;
                    (*scan_prev).next = (*new_block).next;
                }
            }
        }
        release_lock();
    }
}

#[global_allocator]
pub static KERNEL_HEAP: KernelHeap = KernelHeap::new();
