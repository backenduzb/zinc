use crate::boot::limine::MEMORY_MAP_REQUEST;

const PAGE_SIZE: u64 = 4096;

pub fn init() {
    let Some(response) = MEMORY_MAP_REQUEST.response() else {
        crate::drivers::serial::println(
            "ERROR: memory map unavailable"
        );
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
    crate::drivers::serial::print_u64(
        usable_memory / (1024 * 1024)
    );
    crate::drivers::serial::println(" MiB");

    crate::drivers::serial::println(
        "Physical memory manager initialized"
    );
}

pub fn align_up(address: u64) -> u64 {
    (address + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}