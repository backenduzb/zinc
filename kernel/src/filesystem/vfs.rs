use crate::drivers::storage::ide::IdeDevice;
use crate::filesystem::directory;
use crate::filesystem::ext2::Ext2Superblock;
use crate::filesystem::inode::{Ext2Inode, write_inode};
use crate::filesystem::path::lookup_path;

pub struct VfsFile {
    pub inode_number: u32,
    pub inode: Ext2Inode,
    pub offset: u64,
}

pub struct Vfs {
    inode_table_block: u32,
    inode_size: u16,
    block_size: u32,

    inode_bitmap_block: u32,
    inodes_count: u32,
    first_inode: u32,
    free_inodes_count: u32,

    block_bitmap_block: u32,
    blocks_count: u32,
    first_data_block: u32,
    free_blocks_count: u32,
}

impl Vfs {
    pub fn new(
        inode_table_block: u32,
        inode_size: u16,
        superblock: Ext2Superblock,
        inode_bitmap_block: u32,
        block_bitmap_block: u32,
    ) -> Self {
        Self {
            inode_table_block,
            inode_size,
            block_size: superblock.block_size(),

            inode_bitmap_block,
            inodes_count: superblock.inodes_count,
            first_inode: superblock.first_inode,
            free_inodes_count: superblock.free_inodes_count,

            block_bitmap_block,
            blocks_count: superblock.blocks_count,
            first_data_block: superblock.first_data_block,
            free_blocks_count: superblock.free_blocks_count,
        }
    }

    pub fn open(&self, ide: &mut IdeDevice, path: &[u8]) -> Option<VfsFile> {
        let (inode_number, inode) = lookup_path(
            ide,
            self.inode_table_block,
            2,
            path,
            self.inode_size,
            self.block_size,
        )?;

        Some(VfsFile {
            inode_number,
            inode,
            offset: 0,
        })
    }

    pub fn read(&self, ide: &mut IdeDevice, file: &mut VfsFile, buffer: &mut [u8]) -> usize {
        let bytes_read = crate::filesystem::file::read_file(
            ide,
            &file.inode,
            self.block_size,
            file.offset,
            buffer,
        );

        file.offset += bytes_read as u64;

        bytes_read
    }

    pub fn close(&self, _file: VfsFile) {}

    pub fn write(&self, ide: &mut IdeDevice, file: &mut VfsFile, buffer: &[u8]) -> usize {
        if !file.inode.is_regular_file() || buffer.is_empty() {
            return 0;
        }

        let old_size = file.inode.size;

        let bytes_written = crate::filesystem::file::write_file(
            ide,
            &mut file.inode,
            self.block_size,
            file.offset,
            buffer,
            self.block_bitmap_block,
            self.blocks_count,
            self.first_data_block,
            self.free_blocks_count,
        );

        if bytes_written == 0 {
            return 0;
        }

        file.offset += bytes_written as u64;

        if file.inode.size != old_size {
            if !write_inode(
                ide,
                self.inode_table_block,
                file.inode_number,
                self.inode_size,
                self.block_size,
                &file.inode,
            ) {
                return 0;
            }
        }

        bytes_written
    }

    fn find_free_inode(&self, ide: &mut IdeDevice) -> Option<u32> {
        if self.free_inodes_count == 0 {
            return None;
        }

        let sectors_per_block = self.block_size / 512;

        if sectors_per_block == 0 {
            return None;
        }

        for sector in 0..sectors_per_block {
            let lba = self.inode_bitmap_block * sectors_per_block + sector;

            let mut data = [0u8; 512];

            ide.read_sector(lba, &mut data).ok()?;

            for byte_index in 0..512usize {
                let byte = data[byte_index];

                if byte == 0xFF {
                    continue;
                }

                for bit in 0..8u32 {
                    if byte & (1u8 << bit) == 0 {
                        let global_bit = sector as u32 * 4096 + byte_index as u32 * 8 + bit;

                        let inode_number = global_bit + 1;

                        if inode_number < self.first_inode {
                            continue;
                        }

                        if inode_number > self.inodes_count {
                            return None;
                        }

                        return Some(inode_number);
                    }
                }
            }
        }

        None
    }

    fn set_inode_bitmap(&self, ide: &mut IdeDevice, inode_number: u32) -> bool {
        if inode_number == 0 || inode_number > self.inodes_count {
            return false;
        }

        let sectors_per_block = self.block_size / 512;

        if sectors_per_block == 0 {
            return false;
        }

        let bit = inode_number - 1;

        let byte_index = (bit / 8) as usize;
        let bit_index = bit % 8;

        let sector_index = byte_index / 512;
        let offset = byte_index % 512;

        if sector_index >= sectors_per_block as usize {
            return false;
        }

        let lba = self.inode_bitmap_block * sectors_per_block + sector_index as u32;

        let mut sector = [0u8; 512];

        if ide.read_sector(lba, &mut sector).is_err() {
            return false;
        }

        let mask = 1u8 << bit_index;

        if sector[offset] & mask != 0 {
            return false;
        }

        sector[offset] |= mask;

        ide.write_sector(lba, &sector).is_ok()
    }

    pub fn create(
        &self,
        ide: &mut IdeDevice,
        parent_path: &[u8],
        name: &[u8],
    ) -> Result<u32, &'static str> {
        if name.is_empty() || name.len() > 255 {
            return Err("invalid filename");
        }

        if name == b"." || name == b".." || name.contains(&b'/') {
            return Err("invalid filename");
        }

        let (parent_inode_number, parent_inode) = lookup_path(
            ide,
            self.inode_table_block,
            2,
            parent_path,
            self.inode_size,
            self.block_size,
        )
        .ok_or("parent not found")?;

        if !parent_inode.is_directory() {
            return Err("parent is not a directory");
        }

        let directory_size = parent_inode.size as usize;

        if directory_size == 0 {
            return Err("empty directory");
        }

        let mut directory_data = alloc::vec![0u8; directory_size];

        let bytes_read = crate::filesystem::file::read_file(
            ide,
            &parent_inode,
            self.block_size,
            0,
            &mut directory_data,
        );

        if bytes_read != directory_size {
            return Err("failed to read directory");
        }

        if directory::lookup(&directory_data, directory_size, name).is_some() {
            return Err("file already exists");
        }

        let inode_number = self.find_free_inode(ide).ok_or("no free inode")?;

        let new_inode = Ext2Inode {
            mode: 0x81A4,

            uid: 0,
            size: 0,

            atime: 0,
            ctime: 0,
            mtime: 0,
            dtime: 0,

            gid: 0,
            links_count: 1,
            blocks: 0,
            flags: 0,

            block: [0; 15],
        };

        if !write_inode(
            ide,
            self.inode_table_block,
            inode_number,
            self.inode_size,
            self.block_size,
            &new_inode,
        ) {
            return Err("failed to write inode");
        }

        if !directory::add_entry(&mut directory_data, directory_size, inode_number, name, 1) {
            return Err("directory has no free space");
        }

        let directory_block = parent_inode.block[0];

        if directory_block == 0 {
            return Err("directory block missing");
        }

        let mut block_data = alloc::vec![0u8; self.block_size as usize];

        let end = core::cmp::min(self.block_size as usize, directory_data.len());

        block_data[..end].copy_from_slice(&directory_data[..end]);

        let sectors = self.block_size / 512;

        for sector in 0..sectors {
            let offset = sector as usize * 512;

            if ide
                .write_sector(
                    directory_block * sectors + sector,
                    &block_data[offset..offset + 512],
                )
                .is_err()
            {
                return Err("failed to write directory");
            }
        }

        if !self.set_inode_bitmap(ide, inode_number) {
            return Err("failed to update inode bitmap");
        }

        if !self.update_inode_counters(ide) {
            return Err("failed to update inode counters");
        }

        let _ = parent_inode_number;

        Ok(inode_number)
    }

    fn update_inode_counters(&self, ide: &mut IdeDevice) -> bool {
        let mut sb = [0u8; 512];

        if ide.read_sector(2, &mut sb).is_err() {
            return false;
        }

        let free_inodes = u32::from_le_bytes([sb[0x10], sb[0x11], sb[0x12], sb[0x13]]);

        if free_inodes == 0 {
            return false;
        }

        let new_free = free_inodes - 1;

        sb[0x10..0x14].copy_from_slice(&new_free.to_le_bytes());

        if ide.write_sector(2, &sb).is_err() {
            return false;
        }

        let mut gd = [0u8; 512];

        if ide.read_sector(4, &mut gd).is_err() {
            return false;
        }

        let free_group = u16::from_le_bytes([gd[0x0E], gd[0x0F]]);

        if free_group == 0 {
            return false;
        }

        let new_group = free_group - 1;

        gd[0x0E..0x10].copy_from_slice(&new_group.to_le_bytes());

        ide.write_sector(4, &gd).is_ok()
    }
}
