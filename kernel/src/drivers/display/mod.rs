pub mod framebuffer;

use crate::boot::limine::FRAMEBUFFER_REQUEST;

pub fn init() {
    let Some(response) = FRAMEBUFFER_REQUEST.response() else {
        crate::log_boot!("ERROR: framebuffer response unavailable");
        return;
    };

    let framebuffers = response.framebuffers();

    if framebuffers.is_empty() {
        crate::log_boot!("ERROR: no framebuffer available");
        return;
    }

    framebuffer::init(framebuffers[0]);
}

pub fn map_boot_framebuffer(manager: &mut crate::memory::virtualm::VirtualMemoryManager) -> bool {
    let (phys, size) = match framebuffer::boot_phys_range() {
        Some(range) => range,
        None => {
            crate::log_boot!("Framebuffer mapping: unavailable");
            return false;
        }
    };
    if manager.map_mmio(phys, size) {
        crate::log_boot!("Framebuffer mapping: OK");
        true
    } else {
        crate::log_boot!("ERROR: framebuffer mapping failed");
        false
    }
}

pub fn late_init() -> bool {
    framebuffer::late_init()
}
