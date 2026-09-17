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
    crate::drivers::serial::print("=========== PCI =========== ");

    crate::drivers::serial::print_u64(dev.bus as u64);
    crate::drivers::serial::print(":");

    crate::drivers::serial::print_u64(dev.device as u64);
    crate::drivers::serial::print(".");

    crate::drivers::serial::print_u64(dev.function as u64);

    crate::drivers::serial::print(" vendor=");
    crate::drivers::serial::print_hex_u32(dev.vendor_id as u32);

    crate::drivers::serial::print(" device=");
    crate::drivers::serial::print_hex_u32(dev.device_id as u32);

    crate::drivers::serial::print(" class=");
    crate::drivers::serial::print_hex_u32(dev.class as u32);

    crate::drivers::serial::print(" subclass=");
    crate::drivers::serial::print_hex_u32(dev.subclass as u32);

    crate::drivers::serial::print(" command=");
    crate::drivers::serial::print_hex_u32(dev.command as u32);

    crate::drivers::serial::print(" status=");
    crate::drivers::serial::print_hex_u32(dev.status as u32);

    crate::drivers::serial::println("");

    for index in 0..6 {
        print_bar(dev, index);
    }
}

fn print_bar(
    dev: &PciDevice,
    index: u8,
) {
    if let Some(bar) = dev.bar(index) {
        crate::drivers::serial::print("  BAR");
        crate::drivers::serial::print_u64(index as u64);

        if bar.is_io {
            crate::drivers::serial::print(": IO 0x");
        } else {
            crate::drivers::serial::print(": MMIO 0x");
        }

        crate::drivers::serial::print_u64(bar.address);

        crate::drivers::serial::print(" size=0x");
        crate::drivers::serial::print_u64(bar.size);

        if bar.is_64bit {
            crate::drivers::serial::print(" [64-bit]");
        }

        crate::drivers::serial::println("");
    }
}
