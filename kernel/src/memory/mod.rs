pub mod virtualm;
pub mod allocator;
pub mod heap;
pub mod physical;
pub mod test;
pub mod address_space;

pub fn init() {
    physical::init();
}
