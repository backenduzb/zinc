use core::sync::atomic::{AtomicBool, Ordering};

use limine::framebuffer::Framebuffer;

const MODEL_RGB: u8 = 1;
const MAX_PHYSICAL_MEMORY: u64 = 4 * 1024 * 1024 * 1024;

pub struct FramebufferInfo {
    pub address: *mut u8,
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
}

struct EarlyInfo {
    address: u64,
    width: u64,
    height: u64,
    pitch: u64,
    bpp: u16,
    memory_model: u8,
    red_size: u8,
    red_shift: u8,
    green_size: u8,
    green_shift: u8,
    blue_size: u8,
    blue_shift: u8,
    captured: bool,
}

struct BackendState {
    virt_base: u64,
    width: u64,
    height: u64,
    pitch: u64,
    bpp: u16,
    memory_model: u8,
    red_size: u8,
    red_shift: u8,
    green_size: u8,
    green_shift: u8,
    blue_size: u8,
    blue_shift: u8,
    ready: bool,
}

pub struct InfoSnapshot {
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
    pub memory_model: u8,
    pub red_size: u8,
    pub red_shift: u8,
    pub green_size: u8,
    pub green_shift: u8,
    pub blue_size: u8,
    pub blue_shift: u8,
}

static mut EARLY: EarlyInfo = EarlyInfo {
    address: 0,
    width: 0,
    height: 0,
    pitch: 0,
    bpp: 0,
    memory_model: 0,
    red_size: 0,
    red_shift: 0,
    green_size: 0,
    green_shift: 0,
    blue_size: 0,
    blue_shift: 0,
    captured: false,
};

static mut STATE: BackendState = BackendState {
    virt_base: 0,
    width: 0,
    height: 0,
    pitch: 0,
    bpp: 0,
    memory_model: 0,
    red_size: 0,
    red_shift: 0,
    green_size: 0,
    green_shift: 0,
    blue_size: 0,
    blue_shift: 0,
    ready: false,
};

static LOCK: AtomicBool = AtomicBool::new(false);

fn lock() -> u64 {
    let flags: u64;
    unsafe {
        core::arch::asm!("pushfq", "pop {}", out(reg) flags, options(nomem, preserves_flags));
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
    }
    while LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
    flags
}

fn unlock(saved: u64) {
    LOCK.store(false, Ordering::Release);
    if saved & (1 << 9) != 0 {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
        }
    }
}

pub fn init(fb: &Framebuffer) {
    let info = FramebufferInfo {
        address: fb.address() as *mut u8,
        width: fb.width,
        height: fb.height,
        pitch: fb.pitch,
        bpp: fb.bpp,
    };

    unsafe {
        let early = &raw mut EARLY;
        (*early).address = fb.address() as u64;
        (*early).width = fb.width;
        (*early).height = fb.height;
        (*early).pitch = fb.pitch;
        (*early).bpp = fb.bpp;
        (*early).memory_model = fb.memory_model;
        (*early).red_size = fb.red_mask_size;
        (*early).red_shift = fb.red_mask_shift;
        (*early).green_size = fb.green_mask_size;
        (*early).green_shift = fb.green_mask_shift;
        (*early).blue_size = fb.blue_mask_size;
        (*early).blue_shift = fb.blue_mask_shift;
        (*early).captured = true;
    }

    crate::log_boot_raw!("Framebuffer: ");

    crate::log_boot_raw!("{}", info.width);
    crate::log_boot_raw!("x");
    crate::log_boot_raw!("{}", info.height);
    crate::log_boot_raw!(" @ ");
    crate::log_boot_raw!("{}", info.bpp as u64);
    crate::log_boot!("bpp");

    if info.bpp != 32 {
        crate::log_boot!("ERROR: unsupported framebuffer format");
        return;
    }

    if info.address.is_null() {
        crate::log_boot!("ERROR: framebuffer address is null");
        return;
    }

    draw_test(&info);
}

pub fn boot_phys_range() -> Option<(u64, u64)> {
    let (address, width, height, pitch) = unsafe {
        let early = &raw const EARLY;
        if !(*early).captured {
            return None;
        }
        (
            (*early).address,
            (*early).width,
            (*early).height,
            (*early).pitch,
        )
    };
    if address == 0 || width == 0 || height == 0 || pitch == 0 {
        return None;
    }
    if width > 8192 || height > 8192 {
        return None;
    }
    if pitch < width.checked_mul(4)? {
        return None;
    }
    if pitch > 65536 {
        return None;
    }
    let size = height.checked_mul(pitch)?;
    if size == 0 || size > 64 * 1024 * 1024 {
        return None;
    }
    let offset = crate::boot::limine::HHDM_REQUEST.response()?.offset;
    let phys = if address >= offset {
        address.checked_sub(offset)?
    } else {
        address
    };
    if phys == 0 {
        return None;
    }
    let end = phys.checked_add(size)?;
    if end > MAX_PHYSICAL_MEMORY {
        return None;
    }
    Some((phys, size))
}

pub fn late_init() -> bool {
    let early = unsafe {
        let early = &raw const EARLY;
        if !(*early).captured {
            return false;
        }
        InfoSnapshot {
            width: (*early).width,
            height: (*early).height,
            pitch: (*early).pitch,
            bpp: (*early).bpp,
            memory_model: (*early).memory_model,
            red_size: (*early).red_size,
            red_shift: (*early).red_shift,
            green_size: (*early).green_size,
            green_shift: (*early).green_shift,
            blue_size: (*early).blue_size,
            blue_shift: (*early).blue_shift,
        }
    };
    let (phys, _) = match boot_phys_range() {
        Some(range) => range,
        None => return false,
    };
    let offset = match crate::boot::limine::HHDM_REQUEST.response() {
        Some(response) => response.offset,
        None => return false,
    };
    let virt = match offset.checked_add(phys) {
        Some(value) => value,
        None => return false,
    };
    if virt == 0 {
        return false;
    }
    let supported = early.bpp == 32 && early.memory_model == MODEL_RGB;
    let saved = lock();
    unsafe {
        let state = &raw mut STATE;
        (*state).virt_base = virt;
        (*state).width = early.width;
        (*state).height = early.height;
        (*state).pitch = early.pitch;
        (*state).bpp = early.bpp;
        (*state).memory_model = early.memory_model;
        (*state).red_size = early.red_size;
        (*state).red_shift = early.red_shift;
        (*state).green_size = early.green_size;
        (*state).green_shift = early.green_shift;
        (*state).blue_size = early.blue_size;
        (*state).blue_shift = early.blue_shift;
        (*state).ready = supported;
    }
    unlock(saved);
    crate::log_boot_raw!("Framebuffer backend: ");
    crate::log_boot_raw!("{}", early.width);
    crate::log_boot_raw!("x");
    crate::log_boot_raw!("{}", early.height);
    if supported {
        crate::log_boot!(" ready");
    } else {
        crate::log_boot!(" unsupported format");
    }
    true
}

pub fn info_snapshot() -> Option<InfoSnapshot> {
    let saved = lock();
    let snapshot = unsafe {
        let state = &raw const STATE;
        if (*state).width == 0 || (*state).height == 0 || (*state).pitch == 0 {
            None
        } else {
            Some(InfoSnapshot {
                width: (*state).width,
                height: (*state).height,
                pitch: (*state).pitch,
                bpp: (*state).bpp,
                memory_model: (*state).memory_model,
                red_size: (*state).red_size,
                red_shift: (*state).red_shift,
                green_size: (*state).green_size,
                green_shift: (*state).green_shift,
                blue_size: (*state).blue_size,
                blue_shift: (*state).blue_shift,
            })
        }
    };
    unlock(saved);
    snapshot
}

pub fn pack_color(color: u32) -> Option<u32> {
    let saved = lock();
    let masks = unsafe {
        let state = &raw const STATE;
        if !(*state).ready {
            None
        } else {
            Some((
                (*state).red_size,
                (*state).red_shift,
                (*state).green_size,
                (*state).green_shift,
                (*state).blue_size,
                (*state).blue_shift,
            ))
        }
    };
    unlock(saved);
    let (red_size, red_shift, green_size, green_shift, blue_size, blue_shift) = masks?;
    if red_size != 8 || green_size != 8 || blue_size != 8 {
        return None;
    }
    if red_shift >= 32 || green_shift >= 32 || blue_shift >= 32 {
        return None;
    }
    let red = ((color >> 16) & 0xFF) as u64;
    let green = ((color >> 8) & 0xFF) as u64;
    let blue = (color & 0xFF) as u64;
    let packed = (red << red_shift) | (green << green_shift) | (blue << blue_shift);
    if packed > 0xFFFF_FFFF {
        return None;
    }
    Some(packed as u32)
}

fn frame_size(state: &BackendState) -> Option<u64> {
    state.height.checked_mul(state.pitch)
}

pub fn clear(color: u32) -> bool {
    let packed = match pack_color(color) {
        Some(value) => value,
        None => return false,
    };
    let saved = lock();
    let result = unsafe {
        let state = &raw const STATE;
        if !(*state).ready {
            false
        } else if let Some(size) = frame_size(&*state) {
            let mut offset = 0u64;
            let mut ok = true;
            while offset < size {
                let end = match offset.checked_add(4) {
                    Some(value) => value,
                    None => {
                        ok = false;
                        break;
                    }
                };
                if end > size {
                    ok = false;
                    break;
                }
                let target = ((*state).virt_base + offset) as *mut u32;
                core::ptr::write_volatile(target, packed);
                offset = end;
            }
            ok
        } else {
            false
        }
    };
    unlock(saved);
    result
}

pub fn fill_rect(x: u64, y: u64, w: u64, h: u64, color: u32) -> bool {
    if w == 0 || h == 0 {
        return false;
    }
    let packed = match pack_color(color) {
        Some(value) => value,
        None => return false,
    };
    let saved = lock();
    let result = unsafe {
        let state = &raw const STATE;
        if !(*state).ready {
            false
        } else {
            let (x_end, y_end) = match (x.checked_add(w), y.checked_add(h)) {
                (Some(x_end), Some(y_end)) => (x_end, y_end),
                _ => (u64::MAX, u64::MAX),
            };
            if x_end == u64::MAX
                || y_end == u64::MAX
                || x_end > (*state).width
                || y_end > (*state).height
            {
                false
            } else {
                let mut row = y;
                let mut ok = true;
                while row < y_end {
                    let row_base = match row.checked_mul((*state).pitch) {
                        Some(value) => value,
                        None => {
                            ok = false;
                            break;
                        }
                    };
                    let mut col = x;
                    while col < x_end {
                        let pixel = match col.checked_mul(4) {
                            Some(value) => value,
                            None => {
                                ok = false;
                                break;
                            }
                        };
                        let offset = match row_base.checked_add(pixel) {
                            Some(value) => value,
                            None => {
                                ok = false;
                                break;
                            }
                        };
                        let target = ((*state).virt_base + offset) as *mut u32;
                        core::ptr::write_volatile(target, packed);
                        col += 1;
                    }
                    if !ok {
                        break;
                    }
                    row += 1;
                }
                ok
            }
        }
    };
    unlock(saved);
    result
}


pub fn write_row(y: u64, x: u64, pixels: &[u32]) -> bool {
    if pixels.is_empty() {
        return false;
    }

    let saved = lock();
    let result = unsafe {
        let state = &raw const STATE;

        if !(*state).ready || y >= (*state).height {
            false
        } else {
            let count = pixels.len() as u64;

            match (
                x.checked_add(count),
                y.checked_mul((*state).pitch),
                x.checked_mul(4),
                count.checked_mul(4),
            ) {
                (Some(x_end), Some(row_base), Some(start), Some(span))
                    if x_end <= (*state).width =>
                {
                    match (start.checked_add(span), row_base.checked_add(start)) {
                        (Some(row_end), Some(base)) if row_end <= (*state).pitch => {
                            let mut index = 0usize;
                            let mut ok = true;

                            while index < pixels.len() {
                                let offset = match base.checked_add((index as u64) * 4) {
                                    Some(value) => value,
                                    None => {
                                        ok = false;
                                        break;
                                    }
                                };

                                let target = ((*state).virt_base + offset) as *mut u32;
                                core::ptr::write_volatile(target, pixels[index]);
                                index += 1;
                            }

                            ok
                        }
                        _ => false,
                    }
                }
                _ => false,
            }
        }
    };

    unlock(saved);
    result
}

pub fn read_pixel(x: u64, y: u64) -> Option<u32> {
    let saved = lock();
    let result = unsafe {
        let state = &raw const STATE;
        if !(*state).ready {
            None
        } else if x >= (*state).width || y >= (*state).height {
            None
        } else {
            match (
                y.checked_mul((*state).pitch),
                x.checked_mul(4),
                frame_size(&*state),
            ) {
                (Some(row_base), Some(column), Some(size)) => {
                    match (row_base.checked_add(column), size.checked_sub(4)) {
                        (Some(offset), Some(last)) if offset <= last && offset + 4 <= size => {
                            let target = ((*state).virt_base + offset) as *const u32;
                            Some(core::ptr::read_volatile(target))
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }
    };
    unlock(saved);
    result
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
fn put_pixel(fb: &FramebufferInfo, x: u64, y: u64, color: u32) {
    if x >= fb.width || y >= fb.height {
        return;
    }

    let offset = y * fb.pitch + x * 4;

    unsafe {
        let pixel = fb.address.add(offset as usize) as *mut u32;
        pixel.write_volatile(color);
    }
}
