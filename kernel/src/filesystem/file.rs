use crate::drivers::storage::ide::IdeDevice;
use crate::filesystem::inode::Ext2Inode;

const SECTOR_SIZE: u64 = 512;
const DIRECT_BLOCKS: usize = 12;

fn read_block(ide: &mut IdeDevice, block: u32, block_size: u32, buffer: &mut [u8]) -> bool {
    if block_size == 0 {
        return false;
    }
    
    if block == 0 {
        if buffer.len() < block_size as usize {
            return false;
        }
    
        buffer[..block_size as usize].fill(0);
        return true;
    }

    let sectors_per_block = block_size / 512;

    if sectors_per_block == 0 {
        return false;
    }

    if buffer.len() < block_size as usize {
        return false;
    }

    let lba = block as u64 * sectors_per_block as u64;

    for sector in 0..sectors_per_block {
        let mut sector_buffer = [0u8; 512];

        if ide
            .read_sector((lba + sector as u64) as u32, &mut sector_buffer)
            .is_err()
        {
            return false;
        }

        let offset = sector as usize * 512;

        buffer[offset..offset + 512].copy_from_slice(&sector_buffer);
    }

    true
}

fn read_data_block(
    ide: &mut IdeDevice,
    inode: &Ext2Inode,
    block_index: u32,
    block_size: u32,
    buffer: &mut [u8],
) -> bool {
    if block_index < DIRECT_BLOCKS as u32 {
        let block = inode.block[block_index as usize];

        return read_block(ide, block, block_size, buffer);
    }

    let pointers_per_block = block_size / 4;

    if pointers_per_block == 0 {
        return false;
    }

    let indirect_index = block_index - DIRECT_BLOCKS as u32;

    if indirect_index >= pointers_per_block {
        return false;
    }

    let indirect_block = inode.block[12];

    if indirect_block == 0 {
        return false;
    }

    let mut indirect_data = [0u8; 4096];

    if block_size as usize > indirect_data.len() {
        return false;
    }

    if !read_block(ide, indirect_block, block_size, &mut indirect_data) {
        return false;
    }

    let offset = indirect_index as usize * 4;

    let data_block = u32::from_le_bytes([
        indirect_data[offset],
        indirect_data[offset + 1],
        indirect_data[offset + 2],
        indirect_data[offset + 3],
    ]);

    read_block(ide, data_block, block_size, buffer)
}

pub fn read_file(
    ide: &mut IdeDevice,
    inode: &Ext2Inode,
    block_size: u32,
    offset: u64,
    buffer: &mut [u8],
) -> usize {
    if block_size == 0 || buffer.is_empty() {
        return 0;
    }


    let file_size = inode.size as u64;

    if offset >= file_size {
        return 0;
    }

    let available = file_size - offset;

    let requested = core::cmp::min(available, buffer.len() as u64) as usize;

    
    
    let block_size_u64 = block_size as u64;

    let mut total_read = 0usize;

    while total_read < requested {
        let current_offset = offset + total_read as u64;

        let block_index = (current_offset / block_size_u64) as u32;

        if block_index >= 12 {
            crate::drivers::serial::print("read_file: indirect block index=");
            crate::drivers::serial::print_u64(block_index as u64);
            crate::drivers::serial::putc(b'\n');
        }

        let offset_in_block = (current_offset % block_size_u64) as usize;

        let remaining = requested - total_read;

        let bytes_from_block = core::cmp::min(block_size as usize - offset_in_block, remaining);

        let mut block_buffer = [0u8; 4096];

        if block_size as usize > block_buffer.len() {
            break;
        }

        if !read_data_block(ide, inode, block_index, block_size, &mut block_buffer) {
            break;
        }

        buffer[total_read..total_read + bytes_from_block]
            .copy_from_slice(&block_buffer[offset_in_block..offset_in_block + bytes_from_block]);

        total_read += bytes_from_block;
    }

    total_read
}
