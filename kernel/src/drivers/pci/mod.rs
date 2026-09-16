pub mod config;
pub mod device;
pub mod scan;

pub fn init() {
    crate::drivers::serial::println("PCI: scanning ...");

    scan::scan();
    crate::drivers::serial::println("PCI: scan complete");
}