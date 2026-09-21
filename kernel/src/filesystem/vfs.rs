use crate::drivers::storage::ide::IdeDevice;
use crate::filesystem::ext2::Ext2Superblock;
use crate::filesystem::inode::Ext2Inode;
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
}

impl Vfs {
    pub fn new(
        inode_table_block: u32,
        inode_size: u16,
        superblock: Ext2Superblock,
    ) -> Self {
        Self {
            inode_table_block,
            inode_size,
            block_size: superblock.block_size(),
        }
    }

    pub fn open(
        &self,
        ide: &mut IdeDevice,
        path: &[u8],
    ) -> Option<VfsFile> {
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

    pub fn read(
        &self,
        ide: &mut IdeDevice,
        file: &mut VfsFile,
        buffer: &mut [u8],
    ) -> usize {
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

    pub fn close(&self, _file: VfsFile) {
    }
}