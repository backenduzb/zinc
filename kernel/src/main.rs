#![no_std]
#![no_main]

mod arch;
mod boot;
mod drivers;
mod graphics;
mod panic;
mod time;
mod memory;
mod task;

extern crate alloc;

unsafe extern "C" {
    fn c_test();
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    drivers::serial::init();

    drivers::serial::println(
        "ZincOS started!",
    );

    boot::init();
    arch::init();
    time::init();
    memory::init();

    memory::physical::test();

    memory::test::virtual_memory_test();
    time::clock::test();

    unsafe {
        c_test();
    }

    drivers::serial::println(
        "C integration OK",
    );

    drivers::display::init();

}