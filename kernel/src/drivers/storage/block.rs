use alloc::vec;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy)]
pub enum BlockError {
    InvalidBlock,
    BufferTooSmall,
    IoError,
}

pub const BLOCK_SIZE: usize = 512;

pub trait BlockDevice {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), BlockError>;

    fn write_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), BlockError>;

    fn block_size(&self) -> usize;

    fn block_count(&self) -> u64;
}

pub struct RamBlockDevice {
    blocks: Vec<[u8; BLOCK_SIZE]>,
}

impl RamBlockDevice {
    pub fn new(block_count: usize) -> Self {
        Self {
            blocks: vec![[0u8; BLOCK_SIZE]; block_count],
        }
    }
}

impl BlockDevice for RamBlockDevice {
    fn read_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), BlockError> {
        if block >= self.blocks.len() as u64 {
            return Err(BlockError::InvalidBlock);
        }

        if buffer.len() < BLOCK_SIZE {
            return Err(BlockError::BufferTooSmall);
        }

        buffer[..BLOCK_SIZE].copy_from_slice(&self.blocks[block as usize]);

        Ok(())
    }

    fn write_block(&mut self, block: u64, buffer: &mut [u8]) -> Result<(), BlockError> {
        if block >= self.blocks.len() as u64 {
            return Err(BlockError::InvalidBlock);
        }

        if buffer.len() < BLOCK_SIZE {
            return Err(BlockError::BufferTooSmall);
        }

        self.blocks[block as usize].copy_from_slice(&buffer[..BLOCK_SIZE]);

        Ok(())
    }

    fn block_size(&self) -> usize {
        BLOCK_SIZE
    }

    fn block_count(&self) -> u64 {
        self.blocks.len() as u64
    }
}
