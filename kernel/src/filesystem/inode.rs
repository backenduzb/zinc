use crate::drivers::storage::ide::IdeDevice;

#[derive(Clone, Copy)]
pub struct Ext2Inode {
    pub mode: u16,
    pub uid: u16,
    pub size: u32,

    pub atime: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub dtime: u32,

    pub gid: u16,
    pub links_count: u16,
    pub blocks: u32,
    pub flags: u32,

    pub block: [u32; 15],
}

impl Ext2Inode {
    pub fn is_directory(&self) -> bool {
        self.mode & 0xF000 == 0x4000
    }

    pub fn is_regular_file(&self) -> bool {
        self.mode & 0xF000 == 0x8000
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

pub fn parse_inode(buf: &[u8]) -> Option<Ext2Inode> {
    if buf.len() < 128 {
        return None;
    }

    let mut block = [0u32; 15];

    for i in 0..15 {
        block[i] = read_u32(buf, 40 + i * 4);
    }

    Some(Ext2Inode {
        mode: read_u16(buf, 0x00),
        uid: read_u16(buf, 0x02),
        size: read_u32(buf, 0x04),

        atime: read_u32(buf, 0x08),
        ctime: read_u32(buf, 0x0C),
        mtime: read_u32(buf, 0x10),
        dtime: read_u32(buf, 0x14),

        gid: read_u16(buf, 0x18),
        links_count: read_u16(buf, 0x1A),
        blocks: read_u32(buf, 0x1C),
        flags: read_u32(buf, 0x20),

        block,
    })
}

pub fn read_inode(
    ide: &mut IdeDevice,
    inode_table_block: u32,
    inode_number: u32,
    inode_size: u16,
    block_size: u32,
) -> Option<Ext2Inode> {
    if inode_number == 0 {
        return None;
    }

    if inode_size == 0 || block_size == 0 {
        return None;
    }

    let inode_index = inode_number - 1;

    let byte_offset = inode_index as u64 * inode_size as u64;

    let block_offset = byte_offset / block_size as u64;

    let offset_in_block = byte_offset % block_size as u64;

    let inode_block = inode_table_block as u64 + block_offset;

    let sector_size = 512u64;

    let sector_in_block = offset_in_block / sector_size;

    let lba = inode_block * (block_size as u64 / sector_size) + sector_in_block;

    let offset_in_sector = (offset_in_block % sector_size) as usize;

    let mut sector = [0u8; 512];

    ide.read_sector(lba as u32, &mut sector).ok()?;

    let inode_size = inode_size as usize;

    if offset_in_sector + inode_size > sector.len() {
        return None;
    }

    parse_inode(&sector[offset_in_sector..offset_in_sector + inode_size])
}

pub fn write_inode(
    ide: &mut IdeDevice,
    inode_table_block: u32,
    inode_number: u32,
    inode_size: u16,
    block_size: u32,
    inode: &Ext2Inode,
) -> bool {
    if inode_number == 0 || inode_size == 0 || block_size == 0 {
        return false;
    }

    if inode_size as u32 > block_size {
        return false;
    }

    let inode_index = inode_number - 1;

    let byte_offset = inode_index as u64 * inode_size as u64;

    let block_offset = byte_offset / block_size as u64;

    let offset_in_block = byte_offset % block_size as u64;

    let inode_block = inode_table_block as u64 + block_offset;

    let sector_size = 512u64;

    let sector_in_block = offset_in_block / sector_size;

    let lba = inode_block * (block_size as u64 / sector_size) + sector_in_block;

    let offset_in_sector = (offset_in_block % sector_size) as usize;

    let inode_size = inode_size as usize;

    if offset_in_sector + inode_size > 512 {
        return false;
    }

    let mut sector = [0u8; 512];

    if ide.read_sector(lba as u32, &mut sector).is_err() {
        return false;
    }

    let dst = &mut sector[offset_in_sector..offset_in_sector + inode_size];

    dst.fill(0);

    dst[0x00..0x02].copy_from_slice(&inode.mode.to_le_bytes());
    dst[0x02..0x04].copy_from_slice(&inode.uid.to_le_bytes());
    dst[0x04..0x08].copy_from_slice(&inode.size.to_le_bytes());
    dst[0x08..0x0C].copy_from_slice(&inode.atime.to_le_bytes());
    dst[0x0C..0x10].copy_from_slice(&inode.ctime.to_le_bytes());
    dst[0x10..0x14].copy_from_slice(&inode.mtime.to_le_bytes());
    dst[0x14..0x18].copy_from_slice(&inode.dtime.to_le_bytes());
    dst[0x18..0x1A].copy_from_slice(&inode.gid.to_le_bytes());
    dst[0x1A..0x1C].copy_from_slice(&inode.links_count.to_le_bytes());
    dst[0x1C..0x20].copy_from_slice(&inode.blocks.to_le_bytes());
    dst[0x20..0x24].copy_from_slice(&inode.flags.to_le_bytes());

    for i in 0..15 {
        let offset = 40 + i * 4;
        dst[offset..offset + 4].copy_from_slice(&inode.block[i].to_le_bytes());
    }

    ide.write_sector(lba as u32, &sector).is_ok()
}
