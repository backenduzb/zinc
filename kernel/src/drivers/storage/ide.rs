use crate::arch::x86_64::io::{inb, inw, outb};

const DATA: u16 = 0x1F0;
const ERROR: u16 = 0x1F1;
const SECTOR_COUNT: u16 = 0x1F2;
const LBA_LOW: u16 = 0x1F3;
const LBA_MID: u16 = 0x1F4;
const LBA_HIGH: u16 = 0x1F5;
const DRIVE: u16 = 0x1F6;
const STATUS: u16 = 0x1F7;
const COMMAND: u16 = 0x1F7;

const STATUS_BSY: u8 = 0x80;
const STATUS_DRQ: u8 = 0x08;
const STATUS_ERR: u8 = 0x01;

const CMD_WRITE_SECTORS: u8 = 0x30;
const CMD_READ_SECTORS: u8 = 0x20;

pub const SECTOR_SIZE: usize = 512;

#[derive(Debug, Clone, Copy)]
pub enum IdeError {
    BusyTimeout,
    ReadError,
    NoData,
}

pub struct IdeDevice;

impl IdeDevice {
    pub const fn new() -> Self {
        Self
    }

    fn wait_not_busy(&self) -> Result<(), IdeError> {
        for _ in 0..1_000_000 {
            let status = unsafe { inb(STATUS) };

            if status & STATUS_ERR != 0 {
                return Err(IdeError::ReadError);
            }

            if status & STATUS_BSY == 0 {
                return Ok(());
            }
        }

        Err(IdeError::BusyTimeout)
    }

    pub fn read_sector(&mut self, lba: u32, buffer: &mut [u8]) -> Result<(), IdeError> {
        if buffer.len() < SECTOR_SIZE {
            return Err(IdeError::NoData);
        }

        self.wait_not_busy()?;

        unsafe {
            outb(DRIVE, 0xE0 | ((lba >> 24) & 0x0F) as u8);

            outb(SECTOR_COUNT, 1);

            outb(LBA_LOW, (lba & 0xFF) as u8);
            outb(LBA_MID, ((lba >> 8) & 0xFF) as u8);
            outb(LBA_HIGH, ((lba >> 16) & 0xFF) as u8);

            outb(COMMAND, CMD_READ_SECTORS);
        }

        self.wait_not_busy()?;

        let status = unsafe { inb(STATUS) };

        if status & STATUS_ERR != 0 {
            return Err(IdeError::ReadError);
        }

        if status & STATUS_DRQ == 0 {
            return Err(IdeError::NoData);
        }

        for i in 0..256 {
            let word = unsafe { inw(DATA) };

            buffer[i * 2] = (word & 0xFF) as u8;
            buffer[i * 2 + 1] = (word >> 8) as u8;
        }

        Ok(())
    }

    pub fn write_sector(&mut self, lba: u32, buffer: &[u8]) -> Result<(), IdeError> {
        if buffer.len() < SECTOR_SIZE {
            return Err(IdeError::NoData);
        }

        self.wait_not_busy()?;

        unsafe {
            outb(DRIVE, 0xE0 | ((lba >> 24) & 0x0F) as u8);

            outb(SECTOR_COUNT, 1);

            outb(LBA_LOW, (lba & 0xFF) as u8);
            outb(LBA_MID, ((lba >> 8) & 0xFF) as u8);
            outb(LBA_HIGH, ((lba >> 16) & 0xFF) as u8);

            outb(COMMAND, CMD_WRITE_SECTORS);
        }

        for _ in 0..1_000_000 {
            let status = unsafe { inb(STATUS) };

            if status & STATUS_ERR != 0 {
                return Err(IdeError::ReadError);
            }

            if status & STATUS_BSY == 0 && status & STATUS_DRQ != 0 {
                break;
            }
        }

        let status = unsafe { inb(STATUS) };

        if status & STATUS_ERR != 0 {
            return Err(IdeError::ReadError);
        }

        if status & STATUS_DRQ == 0 {
            return Err(IdeError::NoData);
        }

        for i in 0..256 {
            let word = u16::from_le_bytes([buffer[i * 2], buffer[i * 2 + 1]]);

            unsafe {
                crate::arch::x86_64::io::outw(DATA, word);
            }
        }

        self.wait_not_busy()?;

        Ok(())
    }
}
