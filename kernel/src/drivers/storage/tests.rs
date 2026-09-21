use crate::drivers::storage::{
    block::{BlockDevice, BlockError},
    ide::IdeDevice,
};

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
    test_ext2();
    // test_ide();
}

pub fn test_ext2() {
    crate::drivers::serial::println("=========== EXT2 TESTS =========");

    let mut ide = crate::drivers::storage::ide::IdeDevice::new();
    let mut buffer = [0u8; 512];

    if ide.read_sector(2, &mut buffer).is_err() {
        crate::drivers::serial::println("EXT2: SUPERBLOCK READ FAILED");
        return;
    }

    let sb = match crate::filesystem::ext2::parse_superblock(&buffer) {
        Some(sb) => sb,
        None => {
            crate::drivers::serial::println("EXT2: INVALID SUPERBLOCK");
            return;
        }
    };

    crate::drivers::serial::println("EXT2 SUPERBLOCK: PASS");

    let mut gd_buffer = [0u8; 512];

    if ide.read_sector(4, &mut gd_buffer).is_err() {
        crate::drivers::serial::println("EXT2: GROUP DESCRIPTOR READ FAILED");
        return;
    }

    let gd = match crate::filesystem::ext2::parse_group_descriptor(&gd_buffer) {
        Some(gd) => gd,
        None => {
            crate::drivers::serial::println("EXT2: INVALID GROUP DESCRIPTOR");
            return;
        }
    };

    crate::drivers::serial::println("EXT2 GROUP DESCRIPTOR: PASS");

    let inode_table_block = gd.inode_table;

    crate::drivers::serial::print("EXT2 inode table block: ");
    crate::drivers::serial::print_u64(inode_table_block as u64);
    crate::drivers::serial::putc(b'\n');

    let inode_table_lba = inode_table_block * 2;

    let mut inode_sector = [0u8; 512];

    if ide.read_sector(inode_table_lba, &mut inode_sector).is_err() {
        crate::drivers::serial::println("EXT2: INODE READ FAILED");
        return;
    }

    let inode_buf = &inode_sector[256..512];

    let inode = match crate::filesystem::inode::parse_inode(inode_buf) {
        Some(inode) => inode,

        None => {
            crate::drivers::serial::println("EXT2: INVALID INODE");
            return;
        }
    };

    crate::drivers::serial::print("EXT2 inode #2 mode: 0x");
    crate::drivers::serial::print_hex_u32(inode.mode as u32);
    crate::drivers::serial::putc(b'\n');

    crate::drivers::serial::print("EXT2 inode #2 size: ");
    crate::drivers::serial::print_u64(inode.size as u64);
    crate::drivers::serial::putc(b'\n');

    crate::drivers::serial::print("EXT2 inode #2 first block: ");
    crate::drivers::serial::print_u64(inode.block[0] as u64);
    crate::drivers::serial::putc(b'\n');

    if inode.is_directory() {
        crate::drivers::serial::println("EXT2 INODE #2: ROOT DIRECTORY");
        crate::drivers::serial::println("EXT2 INODE #2: PASS");
    } else {
        crate::drivers::serial::println("EXT2 INODE #2: NOT A DIRECTORY");
    }

    let root_block = inode.block[0];
    let root_lba = root_block * 2;

    let mut root_data = [0u8; 1024];

    if ide.read_sector(root_lba, &mut root_data[0..512]).is_err() {
        crate::drivers::serial::println("EXT2: ROOT BLOCK READ FAILED");
        return;
    }

    if ide
        .read_sector(root_lba + 1, &mut root_data[512..1024])
        .is_err()
    {
        crate::drivers::serial::println("EXT2: ROOT BLOCK READ FAILED");
        return;
    }

    crate::drivers::serial::println("EXT2 ROOT DIRECTORY: READ OK");

    let mut offset = 0usize;

    while offset < inode.size as usize {
        let current = &root_data[offset..];

        let entry = match crate::filesystem::directory::parse_entry(current) {
            Some(entry) => entry,
            None => {
                crate::drivers::serial::println("EXT2: INVALID DIRECTORY ENTRY");
                break;
            }
        };

        if entry.inode != 0 {
            if let Some(name) = crate::filesystem::directory::entry_name(current, &entry) {
                crate::drivers::serial::print("  inode=");
                crate::drivers::serial::print_u64(entry.inode as u64);

                crate::drivers::serial::print(" name=");

                for &byte in name {
                    crate::drivers::serial::putc(byte);
                }

                crate::drivers::serial::println("");
            }
        }

        if entry.rec_len == 0 {
            break;
        }

        offset += entry.rec_len as usize;
    }

    let found =
        crate::filesystem::directory::lookup(&root_data, inode.size as usize, b"lost+found");

    match found {
        Some(inode_number) => {
            crate::drivers::serial::print("lookup(lost+found): inode=");
            crate::drivers::serial::print_u64(inode_number as u64);
            crate::drivers::serial::putc(b'\n');
        }

        None => {
            crate::drivers::serial::println("lookup(lost+found): NOT FOUND");
        }
    }

    let lost_found_inode =
        crate::filesystem::inode::read_inode(&mut ide, gd.inode_table, 11, 256, sb.block_size());

    match lost_found_inode {
        Some(lost_inode) => {
            crate::drivers::serial::print("read_inode(11): mode=0x");
            crate::drivers::serial::print_hex_u32(lost_inode.mode as u32);
            crate::drivers::serial::putc(b'\n');

            crate::drivers::serial::print("read_inode(11): size=");
            crate::drivers::serial::print_u64(lost_inode.size as u64);
            crate::drivers::serial::putc(b'\n');

            crate::drivers::serial::print("read_inode(11): first block=");
            crate::drivers::serial::print_u64(lost_inode.block[0] as u64);
            crate::drivers::serial::putc(b'\n');

            if lost_inode.is_directory() {
                crate::drivers::serial::println("read_inode(11): DIRECTORY");
            } else if lost_inode.is_regular_file() {
                crate::drivers::serial::println("read_inode(11): REGULAR FILE");
            } else {
                crate::drivers::serial::println("read_inode(11): OTHER");
            }

            let mut directory_data = [0u8; 12288];

            let bytes_read = crate::filesystem::file::read_file(
                &mut ide,
                &lost_inode,
                sb.block_size(),
                0,
                &mut directory_data,
            );

            let mut offset = 0usize;

            while offset < bytes_read {
                let current = &directory_data[offset..bytes_read];

                let entry = match crate::filesystem::directory::parse_entry(current) {
                    Some(entry) => entry,
                    None => {
                        crate::drivers::serial::println("inode #11 DIRECTORY ENTRY: PARSE FAIL");
                        break;
                    }
                };

                if entry.inode != 0 {
                    if let Some(name) = crate::filesystem::directory::entry_name(current, &entry) {
                        crate::drivers::serial::print("inode #11 entry: ");

                        for &byte in name {
                            crate::drivers::serial::putc(byte);
                        }

                        crate::drivers::serial::print(" -> inode=");
                        crate::drivers::serial::print_u64(entry.inode as u64);
                        crate::drivers::serial::putc(b'\n');
                    }
                }

                if entry.rec_len == 0 {
                    break;
                }

                offset += entry.rec_len as usize;
            }

            if offset == bytes_read {
                crate::drivers::serial::println("inode #11 DIRECTORY PARSE: PASS");
            }

            crate::drivers::serial::print("inode #11 data bytes read: ");
            crate::drivers::serial::print_u64(bytes_read as u64);
            crate::drivers::serial::putc(b'\n');

            if bytes_read == lost_inode.size as usize {
                crate::drivers::serial::println("inode #11 FULL DATA READ: PASS");
            } else {
                crate::drivers::serial::println("inode #11 FULL DATA READ: FAIL");
            }

            for i in 0..15 {
                crate::drivers::serial::print("inode #11 block[");
                crate::drivers::serial::print_u64(i as u64);
                crate::drivers::serial::print("]: ");

                crate::drivers::serial::print_u64(lost_inode.block[i] as u64);

                crate::drivers::serial::putc(b'\n');
            }

            let dot_inode = crate::filesystem::directory::lookup(&directory_data, bytes_read, b".");

            let parent_inode =
                crate::filesystem::directory::lookup(&directory_data, bytes_read, b"..");

            crate::drivers::serial::print("lookup(.) -> inode=");

            crate::drivers::serial::print_u64(dot_inode.unwrap_or(0) as u64);

            crate::drivers::serial::putc(b'\n');

            crate::drivers::serial::print("lookup(..) -> inode=");

            crate::drivers::serial::print_u64(parent_inode.unwrap_or(0) as u64);

            crate::drivers::serial::putc(b'\n');

            if dot_inode == Some(11) && parent_inode == Some(2) {
                crate::drivers::serial::println("EXT2 DIRECTORY LOOKUP: PASS");
            } else {
                crate::drivers::serial::println("EXT2 DIRECTORY LOOKUP: FAIL");
            }
        }

        None => {
            crate::drivers::serial::println("read_inode(11): FAILED");
        }
    }

    let resolved = crate::filesystem::path::lookup_component(
        &mut ide,
        gd.inode_table,
        2,
        b"lost+found",
        256,
        sb.block_size(),
    );

    match resolved {
        Some((resolved_number, resolved_inode)) => {
            crate::drivers::serial::print("PATH lookup(lost+found): inode=");

            crate::drivers::serial::print_u64(resolved_number as u64);

            crate::drivers::serial::putc(b'\n');

            if resolved_number == 11
                && resolved_inode.is_directory()
                && resolved_inode.size == 12288
            {
                crate::drivers::serial::println("EXT2 PATH COMPONENT: PASS");
            } else {
                crate::drivers::serial::println("EXT2 PATH COMPONENT: FAIL");
            }
        }

        None => {
            crate::drivers::serial::println("EXT2 PATH COMPONENT: FAIL");
        }
    }

    let path_result = crate::filesystem::path::lookup_path(
        &mut ide,
        gd.inode_table,
        2,
        b"/lost+found",
        256,
        sb.block_size(),
    );

    match path_result {
        Some((inode_number, inode)) => {
            crate::drivers::serial::print("PATH /lost+found -> inode=");

            crate::drivers::serial::print_u64(inode_number as u64);

            crate::drivers::serial::putc(b'\n');

            if inode_number == 11 && inode.is_directory() && inode.size == 12288 {
                crate::drivers::serial::println("EXT2 PATH RESOLUTION: PASS");
            } else {
                crate::drivers::serial::println("EXT2 PATH RESOLUTION: FAIL");
            }
        }

        None => {
            crate::drivers::serial::println("EXT2 PATH RESOLUTION: FAIL");
        }
    }

    let hello_result = crate::filesystem::path::lookup_path(
        &mut ide,
        gd.inode_table,
        2,
        b"/bin/hello",
        256,
        sb.block_size(),
    );

    match hello_result {
        Some((inode_number, hello_inode)) => {
            crate::drivers::serial::print("PATH /bin/hello -> inode=");

            crate::drivers::serial::print_u64(inode_number as u64);

            crate::drivers::serial::putc(b'\n');

            crate::drivers::serial::print("PATH /bin/hello size=");

            crate::drivers::serial::print_u64(hello_inode.size as u64);

            crate::drivers::serial::putc(b'\n');

            if inode_number == 13 && hello_inode.is_regular_file() && hello_inode.size == 30 {
                crate::drivers::serial::println("EXT2 FILE LOOKUP: PASS");
            } else {
                crate::drivers::serial::println("EXT2 FILE LOOKUP: FAIL");
            }

            let mut hello_data = [0u8; 64];

            let bytes_read = crate::filesystem::file::read_file(
                &mut ide,
                &hello_inode,
                sb.block_size(),
                0,
                &mut hello_data,
            );

            crate::drivers::serial::print("/bin/hello bytes read: ");

            crate::drivers::serial::print_u64(bytes_read as u64);

            crate::drivers::serial::putc(b'\n');

            crate::drivers::serial::print("/bin/hello content: ");

            for i in 0..bytes_read {
                crate::drivers::serial::putc(hello_data[i]);
            }

            crate::drivers::serial::putc(b'\n');

            if bytes_read == hello_inode.size as usize {
                crate::drivers::serial::println("EXT2 FILE READ: PASS");
            } else {
                crate::drivers::serial::println("EXT2 FILE READ: FAIL");
            }
        }

        None => {
            crate::drivers::serial::println("EXT2 FILE LOOKUP: FAIL");
        }
    }
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
