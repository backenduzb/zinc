pub mod pit;
pub mod pic;

pub fn init() {
    pic::init();
    pit::init();
    
    crate::log_boot!("Timer driver initalized");
}