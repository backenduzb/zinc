#![no_std]
#![no_main]

mod arch;
mod boot;
mod drivers;
mod graphics;
mod memory;
mod panic;
mod task;
mod time;
mod exec;
mod elf;
mod syscall;
mod filesystem;
mod tty;
mod log;

extern crate alloc;

unsafe extern "C" {
    fn c_test();
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    drivers::serial::init();

    crate::log_boot!("ZincOS started!");

    boot::init();

    if let Some(rsdp) = crate::boot::limine::rsdp_address() {
        crate::log_boot!("ACPI RSDP found");

        crate::log_boot_raw!("RSDP address: ");
        crate::log_boot_raw!("{}", rsdp);
        crate::log_boot!("");
    } else {
        crate::log_boot!("ERROR: ACPI RSDP not found");
    }

    arch::init();
    time::init();
    memory::init();
    drivers::pci::init();
    drivers::input::init();
    
    if crate::filesystem::context::init() {
        crate::log_vfs!("FILESYSTEM: INIT OK");
    } else {
        crate::log_vfs!("FILESYSTEM: INIT FAILED");
    }
    crate::log_test!("BEFORE PMM TEST");
    crate::tty::init();
    crate::memory::test::address_space_test();
    crate::memory::test::user_address_space_test();
    crate::memory::test::nx_test();
    crate::memory::test::user_page_map_test();
    crate::memory::test::user_page_unmap_test();
    crate::memory::physical::test();
    crate::elf::test::test_loader();
    crate::memory::test::user_stack_test();
    crate::elf::test::test_loader();
    if crate::memory::heap::KERNEL_HEAP.is_initialized() {
        crate::drivers::storage::tests::test_vfs_elf();
        crate::exec::test::test_loader();
    } else {
        crate::log_vfs!("SKIP ELF VFS / EXEC tests before heap (defer after init)");
    }

    crate::log_test!("AFTER PMM TEST");

    // time::clock::test();

    crate::log_test!("AFTER CLOCK TEST");

    unsafe {
        c_test();
    }

    crate::log_boot!("C integration OK");

    drivers::display::init();

    memory::test::virtual_memory_test();
}