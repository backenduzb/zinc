use core::mem::size_of;

use crate::syscall::syscall3;
use crate::syscall::{SYS_GFX_CLEAR, SYS_GFX_INFO, SYS_GFX_RECT, syscall1, syscall2};

pub const COLOR_BG: u32 = 0x0000_0000;
pub const COLOR_PANEL: u32 = 0x0020_1D2B;
pub const COLOR_ACCENT: u32 = 0x00FFFFFF;
pub const COLOR_ACCENT2: u32 = 0x0048_F0C8;

const SYS_GFX_BLIT: u64 = 11;
const LOGO_DATA: &[u8] = include_bytes!("../logo.bin");
const LOGO_HEADER_SIZE: usize = 8;
const LOGO_PIXEL_COUNT: usize = (LOGO_DATA.len() - LOGO_HEADER_SIZE) / 4;

static mut LOGO_FRAME: [u32; LOGO_PIXEL_COUNT] = [0; LOGO_PIXEL_COUNT];

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GfxInfo {
    pub version: u32,
    pub struct_size: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u16,
    pub memory_model: u8,
    pub reserved0: u8,
    pub red_size: u8,
    pub red_shift: u8,
    pub green_size: u8,
    pub green_shift: u8,
    pub blue_size: u8,
    pub blue_shift: u8,
    pub format: u32,
    pub max_blit_bytes: u32,
}

impl GfxInfo {
    pub const fn zeroed() -> Self {
        Self {
            version: 0,
            struct_size: 0,
            width: 0,
            height: 0,
            pitch: 0,
            bpp: 0,
            memory_model: 0,
            reserved0: 0,
            red_size: 0,
            red_shift: 0,
            green_size: 0,
            green_shift: 0,
            blue_size: 0,
            blue_shift: 0,
            format: 0,
            max_blit_bytes: 0,
        }
    }
}

#[repr(C)]
pub struct GfxRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub color: u32,
}

#[repr(C)]
struct GfxBlit {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}


pub fn draw_logo_faded(screen_width: u32, screen_height: u32, alpha: u8) -> bool {
    if LOGO_DATA.len() < LOGO_HEADER_SIZE || (LOGO_DATA.len() - LOGO_HEADER_SIZE) % 4 != 0 {
        return false;
    }

    let width = u32::from_le_bytes([LOGO_DATA[0], LOGO_DATA[1], LOGO_DATA[2], LOGO_DATA[3]]);
    let height = u32::from_le_bytes([LOGO_DATA[4], LOGO_DATA[5], LOGO_DATA[6], LOGO_DATA[7]]);

    if width == 0
        || height == 0
        || (width as usize).checked_mul(height as usize) != Some(LOGO_PIXEL_COUNT)
        || width > screen_width
        || height > screen_height
    {
        return false;
    }

    let x = (screen_width - width) / 2;
    let y = screen_height.saturating_sub(height + 36) / 2;

    let bg_r = ((COLOR_BG >> 16) & 0xFF) as u32;
    let bg_g = ((COLOR_BG >> 8) & 0xFF) as u32;
    let bg_b = (COLOR_BG & 0xFF) as u32;
    let a = alpha as u32;
    let inverse = 255 - a;

    let frame = core::ptr::addr_of_mut!(LOGO_FRAME).cast::<u32>();

    for index in 0..LOGO_PIXEL_COUNT {
        let offset = LOGO_HEADER_SIZE + index * 4;
        let pixel = u32::from_le_bytes([
            LOGO_DATA[offset],
            LOGO_DATA[offset + 1],
            LOGO_DATA[offset + 2],
            LOGO_DATA[offset + 3],
        ]);

        let r = (pixel >> 16) & 0xFF;
        let g = (pixel >> 8) & 0xFF;
        let b = pixel & 0xFF;

        let r = (r * a + bg_r * inverse + 127) / 255;
        let g = (g * a + bg_g * inverse + 127) / 255;
        let b = (b * a + bg_b * inverse + 127) / 255;

        unsafe {
            frame.add(index).write((r << 16) | (g << 8) | b);
        }
    }

    let header = GfxBlit {
        x,
        y,
        w: width,
        h: height,
    };

    syscall3(
        SYS_GFX_BLIT,
        &header as *const GfxBlit as u64,
        size_of::<GfxBlit>() as u64,
        frame as u64,
    ) == 0
}

pub fn get_info(info: &mut GfxInfo) -> bool {
    if size_of::<GfxInfo>() != 40 {
        return false;
    }

    syscall2(
        SYS_GFX_INFO,
        info as *mut GfxInfo as u64,
        size_of::<GfxInfo>() as u64,
    ) == 0
}

pub fn clear(color: u32) -> bool {
    syscall1(SYS_GFX_CLEAR, color as u64) == 0
}

pub fn rect(x: u32, y: u32, w: u32, h: u32, color: u32) -> bool {
    if size_of::<GfxRect>() != 20 {
        return false;
    }

    let rect = GfxRect { x, y, w, h, color };

    syscall2(
        SYS_GFX_RECT,
        &rect as *const GfxRect as u64,
        size_of::<GfxRect>() as u64,
    ) == 0
}

pub fn logo_dimensions() -> Option<(u32, u32)> {
    if LOGO_DATA.len() < LOGO_HEADER_SIZE || (LOGO_DATA.len() - LOGO_HEADER_SIZE) % 4 != 0 {
        return None;
    }

    let width = u32::from_le_bytes(LOGO_DATA[0..4].try_into().ok()?);
    let height = u32::from_le_bytes(LOGO_DATA[4..8].try_into().ok()?);

    if width == 0
        || height == 0
        || (width as usize).checked_mul(height as usize) != Some(LOGO_PIXEL_COUNT)
    {
        return None;
    }

    Some((width, height))
}
