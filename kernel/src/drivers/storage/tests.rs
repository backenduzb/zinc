use crate::drivers::storage::{block::{BlockDevice, BlockError}, ide::IdeDevice};

use super::block::RamBlockDevice;

pub fn test_create() {
    crate::drivers::serial::println("=========== RAMBLOCK TESTS =========");
    crate::drivers::serial::println("=========== RAMBLOCK TESTS =========");
    crate::drivers::serial::println("=========== RAMBLOCK TESTS =========");
    crate::drivers::serial::println("=========== RAMBLOCK TESTS =========");
    crate::drivers::serial::println("=========== RAMBLOCK TESTS =========");

    let mut device = RamBlockDevice::new(4);

    let mut write_buffer = [0u8; 512];

    write_buffer[0] = 0x5A;
    write_buffer[1] = 0xC0;
    write_buffer[511] = 0xFF;

    device.write_block(2, &mut write_buffer).unwrap();

    let mut read_buffer = [0u8; 512];

    device.read_block(2, &mut read_buffer).unwrap();

    assert_eq!(read_buffer[0], 0x5A);
    assert_eq!(read_buffer[1], 0xC0);
    assert_eq!(read_buffer[511], 0xFF);

    crate::drivers::serial::println("RAMBLOCK READ/WRITE: PASS");

    test_errors();
    test_ide();
}

pub fn test_errors() {
    let mut device = RamBlockDevice::new(4);

    let mut buffer = [0u8; 512];

    assert!(matches!(
        device.read_block(4, &mut buffer),
        Err(BlockError::InvalidBlock)
    ));

    let mut small_buffer = [0u8; 128];

    assert!(matches!(
        device.read_block(0, &mut small_buffer),
        Err(BlockError::BufferTooSmall)
    ));

    assert!(matches!(
        device.write_block(4, &mut buffer),
        Err(BlockError::InvalidBlock)
    ));

    assert!(matches!(
        device.write_block(0, &mut small_buffer),
        Err(BlockError::BufferTooSmall)
    ));

    crate::drivers::serial::println("RAMBLOCK ERROR TEST: PASS");
}

pub fn test_ide() {
    let mut ide = IdeDevice::new();
    let mut buffer = [0u8; 512];

    match ide.read_sector(0, &mut buffer) {
        Ok(()) => {
            crate::drivers::serial::println("IDE READ: PASS");

            for i in 0..13 {
                crate::drivers::serial::print_char(buffer[i] as char);
            }

            crate::drivers::serial::println("");
        }

        Err(_) => {
            crate::drivers::serial::println("IDE READ: FAIL");
        }
    }
}