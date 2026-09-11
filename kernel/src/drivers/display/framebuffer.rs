use limine::framebuffer::Framebuffer;

use crate::drivers::serial;

pub struct FramebufferInfo {
    pub address: *mut u8,
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
}

pub fn init(fb: &Framebuffer) {
    let info = FramebufferInfo {
        address: fb.address() as *mut u8,
        width: fb.width,
        height: fb.height,
        pitch: fb.pitch,
        bpp: fb.bpp
    };

    serial::print("Framebuffer: ");
    
    serial::print_u64(info.width);
    serial::print("x");
    serial::print_u64(info.height);
    serial::print(" @ ");
    serial::print_u64(info.bpp as u64);
    serial::println("bpp");

    if info.bpp != 32 {
        serial::println("ERROR: unsupported framebuffer format");
        return;
    }

    if info.address.is_null() {
        serial::println("ERROR: framebuffer address is null");
        return;
    }
    
    draw_test(&info);
}

fn draw_test(fb: &FramebufferInfo) {
    fill(fb, 0x00000000);
    draw_line(fb, 0xFFFFFFFF);
}

fn fill(fb: &FramebufferInfo, color: u32) {
    for y in 0..fb.height {
        for x in 0..fb.width {
            put_pixel(fb, x, y, color);
        }
    }
}

fn draw_line(fb: &FramebufferInfo, color: u32) {
    for x in 0..fb.width {
        put_pixel(fb, x, 10, color);
    } 
}

#[inline]
fn put_pixel(
    fb: &FramebufferInfo,
    x: u64,
    y: u64,
    color: u32,
) {
    if x>=fb.width || y>=fb.height {
        return;
    }

    let offset = y * fb.pitch + x * 4;

    unsafe {
        let pixel = fb.address.add(offset as usize) as *mut u32;
        pixel.write_volatile(color);
    }
}