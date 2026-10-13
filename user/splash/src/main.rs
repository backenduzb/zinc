#![no_std]
#![no_main]

mod animation;
mod graphics;
mod syscall;

use core::panic::PanicInfo;

use animation::run;
use graphics::{get_info, GfxInfo};
use syscall::exit;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    let mut info = GfxInfo::zeroed();

    if !get_info(&mut info) {
        exit(1);
    }

    if info.version != 1 || info.width < 300 || info.height < 180 {
        exit(1);
    }

    if !run(info.width, info.height) {
        exit(1);
    }

    exit(0);
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    exit(1);
}
