use limine::BaseRevision;
use limine::request::{
    FramebufferRequest,
    MemmapRequest,
};

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static MEMORY_MAP_REQUEST: MemmapRequest =
    MemmapRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static FRAMEBUFFER_REQUEST: FramebufferRequest =
    FramebufferRequest::new();


    
pub fn init() {
    if !BASE_REVISION.is_supported() {
        crate::drivers::serial::println(
            "ERROR: unsupported Limine base revision",
        );

        loop {
            core::hint::spin_loop();
        }
    }

    crate::drivers::serial::println("Limine OK");
}