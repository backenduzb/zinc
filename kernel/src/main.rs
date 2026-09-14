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

extern crate alloc;

unsafe extern "C" {
    fn c_test();
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    drivers::serial::init();

    drivers::serial::println("ZincOS started!");

    boot::init();

    if let Some(rsdp) = crate::boot::limine::rsdp_address() {
        crate::drivers::serial::println("ACPI RSDP found");

        crate::drivers::serial::print("RSDP address: ");
        crate::drivers::serial::print_u64(rsdp);
        crate::drivers::serial::println("");
    } else {
        crate::drivers::serial::println("ERROR: ACPI RSDP not found");
    }
    arch::init();
    time::init();
    memory::init();

    memory::physical::test();

    time::clock::test();

    unsafe {
        c_test();
    }

    drivers::serial::println("C integration OK");

    drivers::display::init();

    memory::test::virtual_memory_test();
}
