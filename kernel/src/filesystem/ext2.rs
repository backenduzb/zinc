pub const EXT2_MAGIC: u16 = 0xEF53;

#[derive(Clone, Copy)]
pub struct Ext2Superblock {
    pub inodes_count: u32,
    pub blocks_count: u32,
    pub reserved_blocks_count: u32,
    pub free_blocks_count: u32,
    pub free_inodes_count: u32,
    pub first_data_block: u32,
    pub log_block_size: u32,
    pub first_inode: u32,
    pub inodes_per_group: u32,
    pub magic: u16,
}

#[derive(Clone, Copy)]
pub struct Ext2GroupDescriptor {
    pub block_bitmap: u32,
    pub inode_bitmap: u32,
    pub inode_table: u32,
    pub free_blocks_count: u16,
    pub free_inodes_count: u16,
    pub used_dirs_count: u16,
}

impl Ext2Superblock {
    pub fn block_size(&self) -> u32 {
        1024u32 << self.log_block_size
    }

    pub fn is_valid(&self) -> bool {
        self.magic == EXT2_MAGIC
    }
}

impl Ext2GroupDescriptor {
    pub fn inode_table_block(&self) -> u32 {
        self.inode_table
    }
}

fn read_u16(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

fn read_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

pub fn parse_superblock(buf: &[u8]) -> Option<Ext2Superblock> {
    if buf.len() < 0x58 {
        return None;
    }

    let sb = Ext2Superblock {
        inodes_count: read_u32(buf, 0x00),
        blocks_count: read_u32(buf, 0x04),
        reserved_blocks_count: read_u32(buf, 0x08),
        free_blocks_count: read_u32(buf, 0x0C),
        free_inodes_count: read_u32(buf, 0x10),
        first_data_block: read_u32(buf, 0x14),
        log_block_size: read_u32(buf, 0x18),

        first_inode: read_u32(buf, 0x54),

        inodes_per_group: read_u32(buf, 0x28),

        magic: read_u16(buf, 0x38),
    };

    if !sb.is_valid() {
        return None;
    }

    Some(sb)
}

pub fn parse_group_descriptor(buf: &[u8]) -> Option<Ext2GroupDescriptor> {
    if buf.len() < 18 {
        return None;
    }

    Some(Ext2GroupDescriptor {
        block_bitmap: read_u32(buf, 0x00),
        inode_bitmap: read_u32(buf, 0x04),
        inode_table: read_u32(buf, 0x08),
        free_blocks_count: read_u16(buf, 0x0C),
        free_inodes_count: read_u16(buf, 0x0E),
        used_dirs_count: read_u16(buf, 0x10),
    })
}
