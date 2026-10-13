pub mod abi;
pub mod primites;

use crate::drivers::display::framebuffer;
use abi::{GFX_ABI_VERSION, GFX_FORMAT_XRGB8888, GFX_MAX_BLIT_BYTES, GfxInfo};

pub fn info() -> Option<GfxInfo> {
    let snapshot = framebuffer::info_snapshot()?;
    if snapshot.width == 0 || snapshot.width > u32::MAX as u64 {
        return None;
    }
    if snapshot.height == 0 || snapshot.height > u32::MAX as u64 {
        return None;
    }
    if snapshot.pitch == 0 || snapshot.pitch > u32::MAX as u64 {
        return None;
    }
    Some(GfxInfo {
        version: GFX_ABI_VERSION,
        struct_size: abi::GFX_INFO_SIZE as u32,
        width: snapshot.width as u32,
        height: snapshot.height as u32,
        pitch: snapshot.pitch as u32,
        bpp: snapshot.bpp,
        memory_model: snapshot.memory_model,
        reserved0: 0,
        red_size: snapshot.red_size,
        red_shift: snapshot.red_shift,
        green_size: snapshot.green_size,
        green_shift: snapshot.green_shift,
        blue_size: snapshot.blue_size,
        blue_shift: snapshot.blue_shift,
        format: GFX_FORMAT_XRGB8888,
        max_blit_bytes: GFX_MAX_BLIT_BYTES as u32,
    })
}

pub fn clear(color: u32) -> bool {
    framebuffer::clear(color)
}

pub fn fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) -> bool {
    framebuffer::fill_rect(x as u64, y as u64, w as u64, h as u64, color)
}

pub fn blit_row(y: u32, x: u32, pixels: &[u32]) -> bool {
    framebuffer::write_row(y as u64, x as u64, pixels)
}

pub fn self_test() -> bool {
    let mut ok = true;
    let snapshot = match info() {
        Some(value) => value,
        None => {
            crate::log_test!("GFX SELFTEST: info FAILED");
            return false;
        }
    };
    if snapshot.version != GFX_ABI_VERSION {
        crate::log_test!("GFX SELFTEST: version FAILED");
        return false;
    }
    if snapshot.bpp != 32 {
        crate::log_test!("GFX SELFTEST: format FAILED");
        return false;
    }
    if snapshot.width < 64 || snapshot.height < 64 {
        crate::log_test!("GFX SELFTEST: dimensions FAILED");
        return false;
    }
    crate::log_boot!("GFX SELFTEST: info OK");
    let background = 0x00112233u32;
    if !clear(background) {
        crate::log_test!("GFX SELFTEST: clear FAILED");
        return false;
    }
    let packed_background = match framebuffer::pack_color(background) {
        Some(value) => value,
        None => {
            crate::log_test!("GFX SELFTEST: pack FAILED");
            return false;
        }
    };
    if framebuffer::read_pixel(0, 0) != Some(packed_background) {
        crate::log_test!("GFX SELFTEST: clear readback FAILED");
        ok = false;
    }
    let last_x = snapshot.width as u64 - 1;
    let last_y = snapshot.height as u64 - 1;
    if framebuffer::read_pixel(last_x, last_y) != Some(packed_background) {
        crate::log_test!("GFX SELFTEST: clear corner FAILED");
        ok = false;
    }
    let red = 0x00FF0000u32;
    if !fill_rect(4, 4, 8, 6, red) {
        crate::log_test!("GFX SELFTEST: rect FAILED");
        ok = false;
    }
    let packed_red = match framebuffer::pack_color(red) {
        Some(value) => value,
        None => {
            crate::log_test!("GFX SELFTEST: rect pack FAILED");
            return false;
        }
    };
    if framebuffer::read_pixel(4, 4) != Some(packed_red) {
        crate::log_test!("GFX SELFTEST: rect readback FAILED");
        ok = false;
    }
    if framebuffer::read_pixel(11, 9) != Some(packed_red) {
        crate::log_test!("GFX SELFTEST: rect corner FAILED");
        ok = false;
    }
    if framebuffer::read_pixel(3, 4) != Some(packed_background) {
        crate::log_test!("GFX SELFTEST: rect clip FAILED");
        ok = false;
    }
    let row: [u32; 3] = [
        framebuffer::pack_color(0x0000FF00).unwrap_or(0),
        framebuffer::pack_color(0x000000FF).unwrap_or(0),
        framebuffer::pack_color(0x00FFFFFF).unwrap_or(0),
    ];
    if framebuffer::pack_color(0x0000FF00).is_none() {
        crate::log_test!("GFX SELFTEST: row pack FAILED");
        return false;
    }
    if !framebuffer::write_row(1, 0, &row) {
        crate::log_test!("GFX SELFTEST: row FAILED");
        ok = false;
    }
    if framebuffer::read_pixel(0, 1) != Some(row[0]) {
        crate::log_test!("GFX SELFTEST: row readback FAILED");
        ok = false;
    }
    if framebuffer::read_pixel(2, 1) != Some(row[2]) {
        crate::log_test!("GFX SELFTEST: row end FAILED");
        ok = false;
    }
    if fill_rect(snapshot.width - 2, 0, 4, 1, red) {
        crate::log_test!("GFX SELFTEST: overflow accept FAILED");
        ok = false;
    }
    if fill_rect(0, 0, 0, 1, red) {
        crate::log_test!("GFX SELFTEST: empty accept FAILED");
        ok = false;
    }
    let single = [packed_red];
    if framebuffer::write_row(snapshot.height as u64, 0, &single) {
        crate::log_test!("GFX SELFTEST: row bounds FAILED");
        ok = false;
    }
    if framebuffer::write_row(0, snapshot.width as u64, &single) {
        crate::log_test!("GFX SELFTEST: column bounds FAILED");
        ok = false;
    }
    let empty: [u32; 0] = [];
    if framebuffer::write_row(0, 0, &empty) {
        crate::log_test!("GFX SELFTEST: empty row FAILED");
        ok = false;
    }
    clear(0x00000000);
    fill_rect(0, 10, snapshot.width, 1, 0x00FFFFFF);
    if ok {
        crate::log_boot!("GFX SELFTEST: PASS");
    } else {
        crate::log_test!("GFX SELFTEST: FAIL");
    }
    ok
}
