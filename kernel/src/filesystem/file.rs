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
    if block_size == 0 || block_size as usize > buffer.len() {
        return false;
    }

    if block_index < DIRECT_BLOCKS as u32 {
        return read_block(
            ide,
            inode.block[block_index as usize],
            block_size,
            buffer,
        );
    }

    let pointers_per_block = block_size / 4;

    if pointers_per_block == 0 {
        return false;
    }

    let mut index = block_index - DIRECT_BLOCKS as u32;
    let mut pointer_data = [0u8; 4096];

    if index < pointers_per_block {
        let indirect_block = inode.block[12];

        if indirect_block == 0 {
            buffer[..block_size as usize].fill(0);
            return true;
        }

        if !read_block(
            ide,
            indirect_block,
            block_size,
            &mut pointer_data,
        ) {
            return false;
        }

        let offset = index as usize * 4;
        let data_block = u32::from_le_bytes([
            pointer_data[offset],
            pointer_data[offset + 1],
            pointer_data[offset + 2],
            pointer_data[offset + 3],
        ]);

        return read_block(ide, data_block, block_size, buffer);
    }

    index -= pointers_per_block;

    let double_indirect_capacity =
        pointers_per_block * pointers_per_block;

    if index >= double_indirect_capacity {
        return false;
    }

    let double_indirect_block = inode.block[13];

    if double_indirect_block == 0 {
        buffer[..block_size as usize].fill(0);
        return true;
    }

    if !read_block(
        ide,
        double_indirect_block,
        block_size,
        &mut pointer_data,
    ) {
        return false;
    }

    let first_index = index / pointers_per_block;
    let second_index = index % pointers_per_block;

    let first_offset = first_index as usize * 4;
    let indirect_block = u32::from_le_bytes([
        pointer_data[first_offset],
        pointer_data[first_offset + 1],
        pointer_data[first_offset + 2],
        pointer_data[first_offset + 3],
    ]);

    if indirect_block == 0 {
        buffer[..block_size as usize].fill(0);
        return true;
    }

    if !read_block(
        ide,
        indirect_block,
        block_size,
        &mut pointer_data,
    ) {
        return false;
    }

    let second_offset = second_index as usize * 4;
    let data_block = u32::from_le_bytes([
        pointer_data[second_offset],
        pointer_data[second_offset + 1],
        pointer_data[second_offset + 2],
        pointer_data[second_offset + 3],
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
            crate::log_vfs_raw!("read_file: indirect block index=");
            crate::log_vfs_raw!("{}", block_index as u64);
            crate::log_vfs_raw!("{}", (b'\n') as char);
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

fn write_block(ide: &mut IdeDevice, block: u32, block_size: u32, buffer: &[u8]) -> bool {
    if block == 0 || block_size == 0 {
        return false;
    }

    if block_size % 512 != 0 {
        return false;
    }

    if buffer.len() < block_size as usize {
        return false;
    }

    let sectors_per_block = block_size / 512;

    for sector in 0..sectors_per_block {
        let offset = sector as usize * 512;

        if ide
            .write_sector(
                (block as u64 * sectors_per_block as u64 + sector as u64) as u32,
                &buffer[offset..offset + 512],
            )
            .is_err()
        {
            return false;
        }
    }

    true
}

fn data_block_number(
    ide: &mut IdeDevice,
    inode: &Ext2Inode,
    block_index: u32,
    block_size: u32,
) -> Option<u32> {
    if block_index < DIRECT_BLOCKS as u32 {
        let block = inode.block[block_index as usize];

        if block == 0 {
            return None;
        }

        return Some(block);
    }

    let pointers_per_block = block_size / 4;

    if pointers_per_block == 0 {
        return None;
    }

    let indirect_index = block_index - DIRECT_BLOCKS as u32;

    if indirect_index >= pointers_per_block {
        return None;
    }

    let indirect_block = inode.block[12];

    if indirect_block == 0 {
        return None;
    }

    let mut indirect_data = [0u8; 4096];

    if block_size as usize > indirect_data.len() {
        return None;
    }

    if !read_block(ide, indirect_block, block_size, &mut indirect_data) {
        return None;
    }

    let offset = indirect_index as usize * 4;

    let block = u32::from_le_bytes([
        indirect_data[offset],
        indirect_data[offset + 1],
        indirect_data[offset + 2],
        indirect_data[offset + 3],
    ]);

    if block == 0 { None } else { Some(block) }
}

fn find_free_block(
    ide: &mut IdeDevice,
    block_bitmap_block: u32,
    block_size: u32,
    blocks_count: u32,
    first_data_block: u32,
) -> Option<u32> {
    if block_size == 0 || block_size % 512 != 0 {
        return None;
    }

    let sectors_per_block = block_size / 512;

    if sectors_per_block == 0 {
        return None;
    }

    let bitmap_bytes = block_size as usize;

    let mut bitmap = [0u8; 4096];

    if bitmap_bytes > bitmap.len() {
        return None;
    }

    for sector in 0..sectors_per_block {
        let lba = block_bitmap_block * sectors_per_block + sector;

        let offset = sector as usize * 512;

        if ide
            .read_sector(lba, &mut bitmap[offset..offset + 512])
            .is_err()
        {
            return None;
        }
    }

    let first = first_data_block;

    for block in first..blocks_count {
        let bit = block;

        let byte_index = (bit / 8) as usize;
        let bit_index = bit % 8;

        if byte_index >= bitmap_bytes {
            break;
        }

        let mask = 1u8 << bit_index;

        if bitmap[byte_index] & mask == 0 {
            bitmap[byte_index] |= mask;

            for sector in 0..sectors_per_block {
                let lba = block_bitmap_block * sectors_per_block + sector;

                let offset = sector as usize * 512;

                if ide
                    .write_sector(lba, &bitmap[offset..offset + 512])
                    .is_err()
                {
                    return None;
                }
            }

            return Some(block);
        }
    }

    None
}

fn zero_block(ide: &mut IdeDevice, block: u32, block_size: u32) -> bool {
    let mut buffer = [0u8; 4096];

    if block_size as usize > buffer.len() {
        return false;
    }

    buffer[..block_size as usize].fill(0);

    write_block(ide, block, block_size, &buffer)
}

pub fn write_file(
    ide: &mut IdeDevice,
    inode: &mut Ext2Inode,
    block_size: u32,
    offset: u64,
    data: &[u8],
    block_bitmap_block: u32,
    blocks_count: u32,
    first_data_block: u32,
    _free_blocks_count: u32,
) -> usize {
    if block_size == 0 || data.is_empty() {
        return 0;
    }

    if block_size % 512 != 0 {
        return 0;
    }

    if block_size as usize > 4096 {
        return 0;
    }

    if offset > inode.size as u64 {
        return 0;
    }

    let end_offset = match offset.checked_add(data.len() as u64) {
        Some(value) => value,
        None => return 0,
    };

    let block_size_u64 = block_size as u64;

    let first_block = (offset / block_size_u64) as u32;

    let last_block = ((end_offset.saturating_sub(1)) / block_size_u64) as u32;

    if last_block >= DIRECT_BLOCKS as u32 {
        return 0;
    }
    /*
     * Hozircha faqat 12 ta direct block.
     */

    for block_index in first_block..=last_block {
        if inode.block[block_index as usize] != 0 {
            continue;
        }

        let new_block = match find_free_block(
            ide,
            block_bitmap_block,
            block_size,
            blocks_count,
            first_data_block,
        ) {
            Some(block) => block,
            None => return 0,
        };

        if !zero_block(ide, new_block, block_size) {
            return 0;
        }

        inode.block[block_index as usize] = new_block;
    }

    let mut total_written = 0usize;

    while total_written < data.len() {
        let current_offset = offset + total_written as u64;

        let block_index = (current_offset / block_size_u64) as u32;

        let offset_in_block = (current_offset % block_size_u64) as usize;

        if block_index >= DIRECT_BLOCKS as u32 {
            break;
        }

        let block = inode.block[block_index as usize];

        if block == 0 {
            break;
        }

        let remaining = data.len() - total_written;

        let bytes = core::cmp::min(block_size as usize - offset_in_block, remaining);

        let mut block_buffer = [0u8; 4096];

        if block_size as usize > block_buffer.len() {
            break;
        }

        if !read_block(ide, block, block_size, &mut block_buffer) {
            break;
        }

        block_buffer[offset_in_block..offset_in_block + bytes]
            .copy_from_slice(&data[total_written..total_written + bytes]);

        if !write_block(ide, block, block_size, &block_buffer) {
            break;
        }

        total_written += bytes;
    }

    if total_written > 0 {
        let new_end = offset + total_written as u64;

        if new_end > inode.size as u64 {
            inode.size = new_end as u32;
        }
        let required_blocks = ((inode.size as u64 + block_size_u64 - 1) / block_size_u64) as u32;

        inode.blocks = required_blocks * (block_size / 512);
    }

    total_written
}
