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
