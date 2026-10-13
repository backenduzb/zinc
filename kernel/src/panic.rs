#![cfg_attr(not(test), no_std)]

use core::fmt::Write;
use core::panic::PanicInfo;

struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        crate::log_boot_raw!("{}", s);
        Ok(())
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::log_boot_raw!("KERNEL PANIC: ");

    if let Some(message) = info.message().as_str() {
        crate::log_boot!("{}", message);
    } else {
        let _ = writeln!(SerialWriter, "{}", info.message());
    }

    if let Some(location) = info.location() {
        crate::log_boot_raw!(" at ");
        crate::log_boot_raw!("{}", location.file());
        crate::log_boot_raw!(":");
        crate::log_boot_raw!("{}", location.line() as u64);
        crate::log_boot!("");
    }

    loop {
        core::hint::spin_loop();
    }
}
