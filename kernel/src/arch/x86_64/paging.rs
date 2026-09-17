pub const PAGE_SIZE: u64 = 4096;
const ENTRY_COUNT: usize = 512;
const HUGE_PAGE: u64 = 1 << 7;

const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const USER: u64 = 1 << 2;

const ADDRESS_MASK: u64 = 0x000F_FFFF_FFFF_F000;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn is_present(&self) -> bool {
        self.0 & PRESENT != 0
    }

    pub fn is_huge(&self) -> bool {
        self.0 & HUGE_PAGE != 0
    }

    pub fn set(&mut self, physical_address: u64, writable: bool, user: bool) {
        let mut flags = PRESENT;

        if writable {
            flags |= WRITABLE;
        }

        if user {
            flags |= USER;
        }

        self.0 = (physical_address & ADDRESS_MASK) | flags;
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn address(&self) -> u64 {
        self.0 & ADDRESS_MASK
    }

    pub fn raw(&self) -> u64 {
        self.0
    }

    pub fn set_huge(
        &mut self,
        physical_address: u64,
        writable: bool,
        user: bool
    ) {
        let mut flags = PRESENT | HUGE_PAGE;
        if writable {
            flags |= WRITABLE
        }
        if user {
            flags |= USER;
        }

        self.0 = (physical_address & ADDRESS_MASK) | flags;
    }
}

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; ENTRY_COUNT],
}

impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::empty(); ENTRY_COUNT],
        }
    }

    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }
}

pub fn page_indices(virtual_address: u64) -> (usize, usize, usize, usize) {
    let pml4 = ((virtual_address >> 39) & 0x1FF) as usize;

    let pdpt = ((virtual_address >> 30) & 0x1FF) as usize;

    let pd = ((virtual_address >> 21) & 0x1FF) as usize;

    let pt = ((virtual_address >> 12) & 0x1FF) as usize;

    (pml4, pdpt, pd, pt)
}

pub fn page_offset(virtual_address: u64) -> u64 {
    virtual_address & (PAGE_SIZE - 1)
}

pub unsafe fn load_cr3(pml4_address: u64) {
    unsafe {
        core::arch::asm!(
            "mov cr3, {}",
            in(reg) pml4_address,
            options(nostack, preserves_flags)
        );
    }
}

pub fn read_cr3() -> u64 {
    let value: u64;

    unsafe {
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) value,
            options(nostack, preserves_flags)
        );
    }

    value
}
