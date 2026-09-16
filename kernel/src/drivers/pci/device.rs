use super::config;

#[derive(Clone, Copy)]
pub struct PciBar {
    pub index: u8,
    pub address: u64,
    pub size: u64,
    pub is_io: bool,
    pub is_64bit: bool,
}

#[derive(Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,

    pub vendor_id: u16,
    pub device_id: u16,

    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,

    pub header_type: u8,
}

impl PciDevice {
    pub fn read(bus: u8, device: u8, function: u8) -> Option<Self> {
        let vendor_id = config::read16(bus, device, function, 0x00);

        if vendor_id == 0xFFFF {
            return None;
        }

        let device_id = config::read16(bus, device, function, 0x02);

        let class = config::read8(bus, device, function, 0x0B);

        let subclass = config::read8(bus, device, function, 0x0A);

        let prog_if = config::read8(bus, device, function, 0x09);

        let header_type = config::read8(bus, device, function, 0x0E);

        Some(Self {
            bus,
            device,
            function,
            vendor_id,
            device_id,
            class,
            subclass,
            prog_if,
            header_type,
        })
    }

    pub fn bar(&self, index: u8) -> Option<PciBar> {
        if index >= 6 {
            return None;
        }

        let offset = 0x10 + index * 4;

        let original = config::read32(self.bus, self.device, self.function, offset);

        if original == 0 {
            return None;
        }

        let is_io = (original & 1) != 0;

        if is_io {
            config::write32(self.bus, self.device, self.function, offset, 0xFFFF_FFFF);

            let mask = config::read32(self.bus, self.device, self.function, offset);

            config::write32(self.bus, self.device, self.function, offset, original);

            let masked = mask & 0xFFFF_FFFC;

            if masked == 0 {
                return Some(PciBar {
                    index,
                    address: (original & 0xFFFF_FFFC) as u64,
                    size: 0,
                    is_io: true,
                    is_64bit: false,
                });
            }

            let size = if masked == 0 {
                0
            } else {
                ((!masked).wrapping_add(1)) as u64
            };

            return Some(PciBar {
                index,
                address: (original & 0xFFFF_FFFC) as u64,
                size,
                is_io: true,
                is_64bit: false,
            });
        }

        let memory_type = (original >> 1) & 0x3;
        let is_64bit = memory_type == 0x2;

        let address = if is_64bit {
            if index >= 5 {
                return None;
            }

            let upper = config::read32(self.bus, self.device, self.function, offset + 4);

            ((upper as u64) << 32) | ((original & 0xFFFF_FFF0) as u64)
        } else {
            (original & 0xFFFF_FFF0) as u64
        };

        if is_64bit {
            config::write32(self.bus, self.device, self.function, offset, 0xFFFF_FFFF);

            config::write32(
                self.bus,
                self.device,
                self.function,
                offset + 4,
                0xFFFF_FFFF,
            );

            let low_mask = config::read32(self.bus, self.device, self.function, offset);

            let high_mask = config::read32(self.bus, self.device, self.function, offset + 4);

            let upper = (address >> 32) as u32;

            config::write32(self.bus, self.device, self.function, offset, original);

            config::write32(self.bus, self.device, self.function, offset + 4, upper);

            let mask = ((high_mask as u64) << 32) | ((low_mask & 0xFFFF_FFF0) as u64);

            let size = if mask == 0 {
                0
            } else {
                (!mask).wrapping_add(1)
            };

            return Some(PciBar {
                index,
                address,
                size,
                is_io: false,
                is_64bit: true,
            });
        }

        // 32-bit Memory BAR
        config::write32(self.bus, self.device, self.function, offset, 0xFFFF_FFFF);

        let mask = config::read32(self.bus, self.device, self.function, offset);

        config::write32(self.bus, self.device, self.function, offset, original);

        let masked = mask & 0xFFFF_FFF0;

        let size = if masked == 0 {
            0
        } else {
            ((!masked).wrapping_add(1)) as u64
        };

        Some(PciBar {
            index,
            address,
            size,
            is_io: false,
            is_64bit: false,
        })
    }
}
