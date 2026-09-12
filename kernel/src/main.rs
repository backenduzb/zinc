#![no_std]
#![no_main]

mod arch;
mod boot;
mod drivers;
mod graphics;
mod panic;
mod time;
mod memory;

unsafe extern "C" {
    fn c_test();
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    drivers::serial::init();
    drivers::serial::println("ZincOS started!");

    boot::init();
    arch::init();
    time::init();
    memory::init();
    
    memory::physical::test();
    memory::virtualm::test();
    time::clock::test();
    
    unsafe {
        c_test();
    }

    drivers::serial::println("C integration OK");

    drivers::display::init();
    
    loop {
        arch::x86_64::cpu::halt();
    }
}
