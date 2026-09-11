pub mod framebuffer;

use crate::boot::limine::FRAMEBUFFER_REQUEST;

pub fn init() {
    let Some(response) = FRAMEBUFFER_REQUEST.response() else {
        crate::drivers::serial::println(
            "ERROR: framebuffer response unavailable"
        );
        return;
    };

    let framebuffers = response.framebuffers();

    if framebuffers.is_empty() {
        crate::drivers::serial::println(
            "ERROR: no framebuffer available"
        );
        return;
    }

    framebuffer::init(framebuffers[0]);
}