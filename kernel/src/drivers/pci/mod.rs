pub mod config;
pub mod device;
pub mod scan;

pub fn init() {
    crate::log_boot!("PCI: scanning ...");

    scan::scan();
    crate::log_boot!("PCI: scan complete");
}