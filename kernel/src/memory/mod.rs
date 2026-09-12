pub mod virtualm;
pub mod allocator;
pub mod heap;
pub mod physical;

pub fn init() {
    physical::init();
}
