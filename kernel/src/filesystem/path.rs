use alloc::vec;

use crate::drivers::storage::ide::IdeDevice;
use crate::filesystem::directory::lookup;
use crate::filesystem::inode::{Ext2Inode, read_inode};

pub fn lookup_component(
    ide: &mut IdeDevice,
    inode_table_block: u32,
    inode_number: u32,
    component: &[u8],
    inode_size: u16,
    block_size: u32,
) -> Option<(u32, Ext2Inode)> {
    let inode = read_inode(ide, inode_table_block, inode_number, inode_size, block_size)?;

    if !inode.is_directory() {
        return None;
    }

    let directory_size = inode.size as usize;

    if directory_size == 0 {
        return None;
    }

    let mut directory_data = vec![0u8; directory_size];

    let bytes_read =
        crate::filesystem::file::read_file(ide, &inode, block_size, 0, &mut directory_data);

    if bytes_read != directory_size {
        return None;
    }

    let next_inode_number = lookup(&directory_data, bytes_read, component)?;

    let next_inode = read_inode(
        ide,
        inode_table_block,
        next_inode_number,
        inode_size,
        block_size,
    )?;

    Some((next_inode_number, next_inode))
}

pub fn lookup_path(
    ide: &mut IdeDevice,
    inode_table_block: u32,
    root_inode_number: u32,
    path: &[u8],
    inode_size: u16,
    block_size: u32,
) -> Option<(u32, Ext2Inode)> {
    let mut current_inode_number = root_inode_number;

    let mut start = 0usize;

    while start < path.len() {
        while start < path.len() && path[start] == b'/' {
            start += 1;
        }

        if start >= path.len() {
            break;
        }

        let mut end = start;

        while end < path.len() && path[end] != b'/' {
            end += 1;
        }

        let component = &path[start..end];

        if component == b"." {
            start = end;
            continue;
        }

        if component == b".." {
            let result = lookup_component(
                ide,
                inode_table_block,
                current_inode_number,
                b"..",
                inode_size,
                block_size,
            )?;

            current_inode_number = result.0;
        } else {
            let result = lookup_component(
                ide,
                inode_table_block,
                current_inode_number,
                component,
                inode_size,
                block_size,
            )?;

            current_inode_number = result.0;
        }

        start = end;
    }

    let inode = read_inode(
        ide,
        inode_table_block,
        current_inode_number,
        inode_size,
        block_size,
    )?;

    Some((current_inode_number, inode))
}