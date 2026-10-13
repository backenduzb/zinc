use core::mem::size_of;

pub const GFX_ABI_VERSION: u32 = 1;
pub const GFX_FORMAT_XRGB8888: u32 = 1;
pub const GFX_MAX_DIMENSION: u32 = 8192;
pub const GFX_MAX_BLIT_BYTES: usize = 16 * 1024 * 1024;

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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GfxRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub color: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GfxBlit {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub const GFX_INFO_SIZE: usize = size_of::<GfxInfo>();
pub const GFX_RECT_SIZE: usize = size_of::<GfxRect>();
pub const GFX_BLIT_SIZE: usize = size_of::<GfxBlit>();

fn store_u32(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn load_u32(input: &[u8], offset: usize) -> u32 {
    let mut raw = [0u8; 4];
    raw.copy_from_slice(&input[offset..offset + 4]);
    u32::from_le_bytes(raw)
}

impl GfxInfo {
    pub fn to_bytes(&self) -> [u8; 40] {
        let mut out = [0u8; 40];
        store_u32(&mut out, 0, self.version);
        store_u32(&mut out, 4, self.struct_size);
        store_u32(&mut out, 8, self.width);
        store_u32(&mut out, 12, self.height);
        store_u32(&mut out, 16, self.pitch);
        out[20..22].copy_from_slice(&self.bpp.to_le_bytes());
        out[22] = self.memory_model;
        out[23] = self.reserved0;
        out[24] = self.red_size;
        out[25] = self.red_shift;
        out[26] = self.green_size;
        out[27] = self.green_shift;
        out[28] = self.blue_size;
        out[29] = self.blue_shift;
        store_u32(&mut out, 32, self.format);
        store_u32(&mut out, 36, self.max_blit_bytes);
        out
    }
}

impl GfxRect {
    pub fn from_bytes(input: &[u8]) -> Option<Self> {
        if input.len() < GFX_RECT_SIZE {
            return None;
        }
        Some(Self {
            x: load_u32(input, 0),
            y: load_u32(input, 4),
            w: load_u32(input, 8),
            h: load_u32(input, 12),
            color: load_u32(input, 16),
        })
    }

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut out = [0u8; 20];
        store_u32(&mut out, 0, self.x);
        store_u32(&mut out, 4, self.y);
        store_u32(&mut out, 8, self.w);
        store_u32(&mut out, 12, self.h);
        store_u32(&mut out, 16, self.color);
        out
    }
}

impl GfxBlit {
    pub fn from_bytes(input: &[u8]) -> Option<Self> {
        if input.len() < GFX_BLIT_SIZE {
            return None;
        }
        Some(Self {
            x: load_u32(input, 0),
            y: load_u32(input, 4),
            w: load_u32(input, 8),
            h: load_u32(input, 12),
        })
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        store_u32(&mut out, 0, self.x);
        store_u32(&mut out, 4, self.y);
        store_u32(&mut out, 8, self.w);
        store_u32(&mut out, 12, self.h);
        out
    }
}
