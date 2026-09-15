pub mod pit;
pub mod pic;

pub fn init() {
    pic::init();
    pit::init();
    
    crate::drivers::serial::println("Timer driver initalized");
}