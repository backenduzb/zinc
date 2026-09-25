use super::device::PciDevice;

pub fn scan() {
    for bus in 0u16..=255 {
        for device in 0u8..32 {
            for function in 0u8..8 {
                if let Some(dev) = PciDevice::read(bus as u8, device, function) {
                    print_device(&dev);
                }
            }
        }
    }
}

fn print_device(dev: &PciDevice) {
    crate::log_boot_raw!("=========== PCI =========== ");

    crate::log_boot_raw!("{}", dev.bus as u64);
    crate::log_boot_raw!(":");

    crate::log_boot_raw!("{}", dev.device as u64);
    crate::log_boot_raw!(".");

    crate::log_boot_raw!("{}", dev.function as u64);

    crate::log_boot_raw!(" vendor=");
    crate::log_boot_raw!("{:08X}", dev.vendor_id as u32);

    crate::log_boot_raw!(" device=");
    crate::log_boot_raw!("{:08X}", dev.device_id as u32);

    crate::log_boot_raw!(" class=");
    crate::log_boot_raw!("{:08X}", dev.class as u32);

    crate::log_boot_raw!(" subclass=");
    crate::log_boot_raw!("{:08X}", dev.subclass as u32);

    crate::log_boot_raw!(" command=");
    crate::log_boot_raw!("{:08X}", dev.command as u32);

    crate::log_boot_raw!(" status=");
    crate::log_boot_raw!("{:08X}", dev.status as u32);

    crate::log_boot!("");

    for index in 0..6 {
        print_bar(dev, index);
    }
}

fn print_bar(
    dev: &PciDevice,
    index: u8,
) {
    if let Some(bar) = dev.bar(index) {
        crate::log_boot_raw!("  BAR");
        crate::log_boot_raw!("{}", index as u64);

        if bar.is_io {
            crate::log_boot_raw!(": IO 0x");
        } else {
            crate::log_boot_raw!(": MMIO 0x");
        }

        crate::log_boot_raw!("{}", bar.address);

        crate::log_boot_raw!(" size=0x");
        crate::log_boot_raw!("{}", bar.size);

        if bar.is_64bit {
            crate::log_boot_raw!(" [64-bit]");
        }

        crate::log_boot!("");
    }
}
