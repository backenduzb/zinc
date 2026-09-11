pub mod pit;
pub mod pic;

pub fn init() {
    pit::init();
    pic::init();
    
    crate::drivers::serial::println("Timer driver initalized");
}