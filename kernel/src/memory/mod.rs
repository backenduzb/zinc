pub mod virtualm;
pub mod allocator;
pub mod heap;
pub mod physical;
pub mod test;

pub fn init() {
    physical::init();
}
