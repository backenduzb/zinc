use limine::BaseRevision;
use limine::request::{
    ExecutableAddressRequest,
    FramebufferRequest,
    MemmapRequest,
    HhdmRequest,
};

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static MEMORY_MAP_REQUEST: MemmapRequest =
    MemmapRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

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

pub fn physical_to_virtual(
    physical_address: u64,
) -> Option<u64> {
    let response = HHDM_REQUEST.response()?;
    Some(response.offset + physical_address)
}