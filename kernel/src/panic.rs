use core::fmt::Write;
use core::panic::PanicInfo;

struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        crate::drivers::serial::print(s);
        Ok(())
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::drivers::serial::print("KERNEL PANIC: ");

    if let Some(message) = info.message().as_str() {
        crate::drivers::serial::println(message);
    } else {
        // `expect()`, `unwrap()` and formatted panics don't expose a `&str`.
        // Display them via fmt instead of hiding them as "unknown panic".
        let _ = writeln!(SerialWriter, "{}", info.message());
    }

    if let Some(location) = info.location() {
        crate::drivers::serial::print(" at ");
        crate::drivers::serial::print(location.file());
        crate::drivers::serial::print(":");
        crate::drivers::serial::print_u64(location.line() as u64);
        crate::drivers::serial::println("");
    }

    loop {
        core::hint::spin_loop();
    }
}