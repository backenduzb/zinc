use alloc::collections::btree_map::Values;

use crate::{arch::x86_64::{io::{inl, outl}, paging::PageTable}, drivers::pci::device};

const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

#[inline]
fn config_address(
    bus: u8,
    device: u8,
    function: u8,
    offset: u8,
) -> u32 {
    (1 << 31)
    | ((bus as u32) << 16)
    | ((device as u32) << 11)
    | ((function as u32) << 8)
    | ((offset as u32) & 0xFC)
}

#[inline]
pub fn read32(
    bus: u8,
    device: u8,
    function: u8,
    offset: u8,
) -> u32 {
    let address = config_address(
        bus,
        device,
        function,
        offset,
    );

    unsafe {
        outl(CONFIG_ADDRESS, address);
        inl(CONFIG_DATA)
    }
}

#[inline]
pub fn read16 (
    bus: u8,
    device: u8,
    function: u8,
    offset: u8,
) -> u16 {
    let value = read32(
        bus,
        device,
        function,
        offset & 0xFC
    );

    let shift = ((offset & 2) * 8) as u32;

    ((value >> shift) & 0xFFFF) as u16
}

#[inline]
pub fn read8(
    bus: u8,
    device: u8,
    function: u8,
    offset: u8
) -> u8 {
    let value = read32(
        bus,
        device,
        function,
        offset & 0xFC
    );

    let shift = ((offset & 3) * 8) as u32;
    
    ((value >> shift) & 0xFF) as u8
}

#[inline]
pub fn write32(
    bus: u8,
    device: u8,
    function: u8,
    offset: u8,
    value: u32,
) {
    let address = config_address(bus, device, function, offset);

    unsafe {
        outl(CONFIG_ADDRESS, address);
        outl(CONFIG_DATA, value);
    }
}