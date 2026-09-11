use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::drivers::serial::print("KERNEL PANIC: ");

    if let Some(message) = info.message().as_str() {
        crate::drivers::serial::println(message);
    } else {
        crate::drivers::serial::println("unknown panic");
    }

    loop {
        core::hint::spin_loop();
    }
}