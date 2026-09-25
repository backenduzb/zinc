use crate::drivers::storage::{
    block::{BlockDevice, BlockError},
    ide::IdeDevice,
};

use super::block::RamBlockDevice;

pub fn test_create() {
    crate::log_test!("=========== RAMBLOCK TESTS =========");
    crate::log_test!("=========== RAMBLOCK TESTS =========");
    crate::log_test!("=========== RAMBLOCK TESTS =========");
    crate::log_test!("=========== RAMBLOCK TESTS =========");
    crate::log_test!("=========== RAMBLOCK TESTS =========");

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

    crate::log_vfs!("RAMBLOCK READ/WRITE: PASS");

    test_errors();
    test_ext2();
    // test_ide();
    test_wait();
}

pub fn test_wait() {
    crate::log_test!("=== WAIT TEST ===");

    let mut frame_allocator =
        crate::memory::physical::FrameAllocator::new();

    let parent_pid;
    let child_pid;

    unsafe {
        let pm = crate::task::process_manager();

        parent_pid = match (*pm).create_user(
            &mut frame_allocator,
            None,
        ) {
            Some(pid) => pid,
            None => {
                crate::log_test!("WAIT TEST: parent create FAILED");
                return;
            }
        };

        child_pid = match (*pm).create_user(
            &mut frame_allocator,
            Some(parent_pid),
        ) {
            Some(pid) => pid,
            None => {
                crate::log_test!("WAIT TEST: child create FAILED");
                return;
            }
        };
    }

    crate::log_test_raw!("WAIT TEST: parent PID=");
    crate::log_test_raw!("{}", parent_pid);
    crate::log_test!("");

    crate::log_test_raw!("WAIT TEST: child PID=");
    crate::log_test_raw!("{}", child_pid);
    crate::log_test!("");

    unsafe {
        let pm = crate::task::process_manager();

        if !(*pm).terminate(child_pid) {
            crate::log_test!("WAIT TEST: child terminate FAILED");
            return;
        }
    }

    crate::log_test!("WAIT TEST: child terminated");

    let found = unsafe {
        let pm = crate::task::process_manager();

        (*pm).find_terminated_child(parent_pid)
    };

    if found != Some(child_pid) {
        crate::log_test!("WAIT TEST: child lookup FAILED");
        return;
    }

    crate::log_test!("WAIT TEST: terminated child found");

    let reaped = unsafe {
        let pm = crate::task::process_manager();

        (*pm).reap(
            child_pid,
            &mut frame_allocator,
        )
    };

    if !reaped {
        crate::log_exec!("WAIT TEST: reap FAILED");
        return;
    }

    crate::log_exec_raw!("WAIT TEST: reaped child PID=");
    crate::log_test_raw!("{}", child_pid);
    crate::log_test!("");

    let exists = unsafe {
        let pm = crate::task::process_manager();
        (*pm).get(child_pid).is_some()
    };

    if exists {
        crate::log_test!("WAIT TEST: child still exists FAIL");
        return;
    }

    crate::log_test!("WAIT TEST: PASS");
}

pub fn test_ext2() {
    crate::log_test!("=========== EXT2 TESTS =========");

    let mut ide = crate::drivers::storage::ide::IdeDevice::new();
    let mut buffer = [0u8; 512];

    if ide.read_sector(2, &mut buffer).is_err() {
        crate::log_vfs!("EXT2: SUPERBLOCK READ FAILED");
        return;
    }

    let sb = match crate::filesystem::ext2::parse_superblock(&buffer) {
        Some(sb) => sb,
        None => {
            crate::log_test!("EXT2: INVALID SUPERBLOCK");
            return;
        }
    };

    crate::log_test!("EXT2 SUPERBLOCK: PASS");

    let mut gd_buffer = [0u8; 512];

    if ide.read_sector(4, &mut gd_buffer).is_err() {
        crate::log_vfs!("EXT2: GROUP DESCRIPTOR READ FAILED");
        return;
    }

    let gd = match crate::filesystem::ext2::parse_group_descriptor(&gd_buffer) {
        Some(gd) => gd,
        None => {
            crate::log_test!("EXT2: INVALID GROUP DESCRIPTOR");
            return;
        }
    };

    crate::log_test!("EXT2 GROUP DESCRIPTOR: PASS");

    let inode_table_block = gd.inode_table;

    crate::log_vfs_raw!("EXT2 inode table block: ");
    crate::log_test_raw!("{}", inode_table_block as u64);
    crate::log_test_raw!("{}", (b'\n') as char);

    let inode_table_lba = inode_table_block * 2;

    let mut inode_sector = [0u8; 512];

    if ide.read_sector(inode_table_lba, &mut inode_sector).is_err() {
        crate::log_vfs!("EXT2: INODE READ FAILED");
        return;
    }

    let inode_buf = &inode_sector[256..512];

    let inode = match crate::filesystem::inode::parse_inode(inode_buf) {
        Some(inode) => inode,

        None => {
            crate::log_vfs!("EXT2: INVALID INODE");
            return;
        }
    };

    crate::log_vfs_raw!("EXT2 inode #2 mode: 0x");
    crate::log_test_raw!("{:08X}", inode.mode as u32);
    crate::log_test_raw!("{}", (b'\n') as char);

    crate::log_vfs_raw!("EXT2 inode #2 size: ");
    crate::log_test_raw!("{}", inode.size as u64);
    crate::log_test_raw!("{}", (b'\n') as char);

    crate::log_vfs_raw!("EXT2 inode #2 first block: ");
    crate::log_test_raw!("{}", inode.block[0] as u64);
    crate::log_test_raw!("{}", (b'\n') as char);

    if inode.is_directory() {
        crate::log_vfs!("EXT2 INODE #2: ROOT DIRECTORY");
        crate::log_vfs!("EXT2 INODE #2: PASS");
    } else {
        crate::log_vfs!("EXT2 INODE #2: NOT A DIRECTORY");
    }

    let root_block = inode.block[0];
    let root_lba = root_block * 2;

    let mut root_data = [0u8; 1024];

    if ide.read_sector(root_lba, &mut root_data[0..512]).is_err() {
        crate::log_vfs!("EXT2: ROOT BLOCK READ FAILED");
        return;
    }

    if ide
        .read_sector(root_lba + 1, &mut root_data[512..1024])
        .is_err()
    {
        crate::log_vfs!("EXT2: ROOT BLOCK READ FAILED");
        return;
    }

    crate::log_vfs!("EXT2 ROOT DIRECTORY: READ OK");

    let mut offset = 0usize;

    while offset < inode.size as usize {
        let current = &root_data[offset..];

        let entry = match crate::filesystem::directory::parse_entry(current) {
            Some(entry) => entry,
            None => {
                crate::log_vfs!("EXT2: INVALID DIRECTORY ENTRY");
                break;
            }
        };

        if entry.inode != 0 {
            if let Some(name) = crate::filesystem::directory::entry_name(current, &entry) {
                crate::log_vfs_raw!("  inode=");
                crate::log_test_raw!("{}", entry.inode as u64);

                crate::log_test_raw!(" name=");

                for &byte in name {
                    crate::log_test_raw!("{}", (byte) as char);
                }

                crate::log_test!("");
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
            crate::log_vfs_raw!("lookup(lost+found): inode=");
            crate::log_test_raw!("{}", inode_number as u64);
            crate::log_test_raw!("{}", (b'\n') as char);
        }

        None => {
            crate::log_test!("lookup(lost+found): NOT FOUND");
        }
    }

    let lost_found_inode =
        crate::filesystem::inode::read_inode(&mut ide, gd.inode_table, 11, 256, sb.block_size());

    match lost_found_inode {
        Some(lost_inode) => {
            crate::log_vfs_raw!("read_inode(11): mode=0x");
            crate::log_test_raw!("{:08X}", lost_inode.mode as u32);
            crate::log_test_raw!("{}", (b'\n') as char);

            crate::log_vfs_raw!("read_inode(11): size=");
            crate::log_test_raw!("{}", lost_inode.size as u64);
            crate::log_test_raw!("{}", (b'\n') as char);

            crate::log_vfs_raw!("read_inode(11): first block=");
            crate::log_test_raw!("{}", lost_inode.block[0] as u64);
            crate::log_test_raw!("{}", (b'\n') as char);

            if lost_inode.is_directory() {
                crate::log_vfs!("read_inode(11): DIRECTORY");
            } else if lost_inode.is_regular_file() {
                crate::log_vfs!("read_inode(11): REGULAR FILE");
            } else {
                crate::log_vfs!("read_inode(11): OTHER");
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
                        crate::log_vfs!("inode #11 DIRECTORY ENTRY: PARSE FAIL");
                        break;
                    }
                };

                if entry.inode != 0 {
                    if let Some(name) = crate::filesystem::directory::entry_name(current, &entry) {
                        crate::log_vfs_raw!("inode #11 entry: ");

                        for &byte in name {
                            crate::log_test_raw!("{}", (byte) as char);
                        }

                        crate::log_vfs_raw!(" -> inode=");
                        crate::log_test_raw!("{}", entry.inode as u64);
                        crate::log_test_raw!("{}", (b'\n') as char);
                    }
                }

                if entry.rec_len == 0 {
                    break;
                }

                offset += entry.rec_len as usize;
            }

            if offset == bytes_read {
                crate::log_vfs!("inode #11 DIRECTORY PARSE: PASS");
            }

            crate::log_vfs_raw!("inode #11 data bytes read: ");
            crate::log_test_raw!("{}", bytes_read as u64);
            crate::log_test_raw!("{}", (b'\n') as char);

            if bytes_read == lost_inode.size as usize {
                crate::log_vfs!("inode #11 FULL DATA READ: PASS");
            } else {
                crate::log_vfs!("inode #11 FULL DATA READ: FAIL");
            }

            for i in 0..15 {
                crate::log_vfs_raw!("inode #11 block[");
                crate::log_test_raw!("{}", i as u64);
                crate::log_test_raw!("]: ");

                crate::log_test_raw!("{}", lost_inode.block[i] as u64);

                crate::log_test_raw!("{}", (b'\n') as char);
            }

            let dot_inode = crate::filesystem::directory::lookup(&directory_data, bytes_read, b".");

            let parent_inode =
                crate::filesystem::directory::lookup(&directory_data, bytes_read, b"..");

            crate::log_vfs_raw!("lookup(.) -> inode=");

            crate::log_test_raw!("{}", dot_inode.unwrap_or(0) as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            crate::log_vfs_raw!("lookup(..) -> inode=");

            crate::log_test_raw!("{}", parent_inode.unwrap_or(0) as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            if dot_inode == Some(11) && parent_inode == Some(2) {
                crate::log_vfs!("EXT2 DIRECTORY LOOKUP: PASS");
            } else {
                crate::log_vfs!("EXT2 DIRECTORY LOOKUP: FAIL");
            }
        }

        None => {
            crate::log_vfs!("read_inode(11): FAILED");
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
            crate::log_vfs_raw!("PATH lookup(lost+found): inode=");

            crate::log_test_raw!("{}", resolved_number as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            if resolved_number == 11
                && resolved_inode.is_directory()
                && resolved_inode.size == 12288
            {
                crate::log_test!("EXT2 PATH COMPONENT: PASS");
            } else {
                crate::log_test!("EXT2 PATH COMPONENT: FAIL");
            }
        }

        None => {
            crate::log_test!("EXT2 PATH COMPONENT: FAIL");
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
            crate::log_vfs_raw!("PATH /lost+found -> inode=");

            crate::log_test_raw!("{}", inode_number as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            if inode_number == 11 && inode.is_directory() && inode.size == 12288 {
                crate::log_test!("EXT2 PATH RESOLUTION: PASS");
            } else {
                crate::log_test!("EXT2 PATH RESOLUTION: FAIL");
            }
        }

        None => {
            crate::log_test!("EXT2 PATH RESOLUTION: FAIL");
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
            crate::log_vfs_raw!("PATH /bin/hello -> inode=");

            crate::log_test_raw!("{}", inode_number as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            crate::log_test_raw!("PATH /bin/hello size=");

            crate::log_test_raw!("{}", hello_inode.size as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            if hello_inode.is_regular_file() {
                crate::log_vfs!("EXT2 FILE LOOKUP: PASS");
            } else {
                crate::log_vfs!("EXT2 FILE LOOKUP: FAIL");
            }

            let mut hello_data = [0u8; 8192];

            let bytes_read = crate::filesystem::file::read_file(
                &mut ide,
                &hello_inode,
                sb.block_size(),
                0,
                &mut hello_data,
            );

            crate::log_vfs_raw!("/bin/hello bytes read: ");
            crate::log_test_raw!("{}", bytes_read as u64);
            crate::log_test_raw!("{}", (b'\n') as char);

            if bytes_read == hello_inode.size as usize {
                crate::log_vfs!("EXT2 FILE READ: PASS");
            } else {
                crate::log_vfs!("EXT2 FILE READ: FAIL");
            }
        }

        None => {
            crate::log_vfs!("EXT2 FILE LOOKUP: FAIL");
        }
    }

    let big_result = crate::filesystem::path::lookup_path(
        &mut ide,
        gd.inode_table_block(),
        2,
        b"/test/big",
        256,
        sb.block_size(),
    );

    match big_result {
        Some((inode_number, big_inode)) => {
            crate::log_vfs_raw!("PATH /test/big -> inode=");

            crate::log_test_raw!("{}", inode_number as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            crate::log_test_raw!("/test/big size=");

            crate::log_test_raw!("{}", big_inode.size as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            crate::log_vfs_raw!("/test/big indirect block=");

            crate::log_test_raw!("{}", big_inode.block[12] as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            let mut big_data = [0u8; 13312];

            let bytes_read = crate::filesystem::file::read_file(
                &mut ide,
                &big_inode,
                sb.block_size(),
                0,
                &mut big_data,
            );

            crate::log_vfs_raw!("/test/big bytes read: ");

            crate::log_test_raw!("{}", bytes_read as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            if bytes_read == big_inode.size as usize {
                crate::log_vfs!("EXT2 INDIRECT READ: PASS");
            } else {
                crate::log_vfs!("EXT2 INDIRECT READ: FAIL");
            }

            for i in 0..15 {
                crate::log_test_raw!("/test/big block[");

                crate::log_test_raw!("{}", i as u64);

                crate::log_test_raw!("]=");

                crate::log_test_raw!("{}", big_inode.block[i] as u64);

                crate::log_test_raw!("{}", (b'\n') as char);
            }
        }

        None => {
            crate::log_test!("PATH /test/big: FAIL");
        }
    }

    let vfs = crate::filesystem::vfs::Vfs::new(gd.inode_table_block(), 256, sb);

    let mut file = match vfs.open(&mut ide, b"/bin/hello") {
        Some(file) => {
            crate::log_vfs_raw!("VFS OPEN /bin/hello: inode=");

            crate::log_test_raw!("{}", file.inode_number as u64);

            crate::log_test_raw!("{}", (b'\n') as char);

            file
        }

        None => {
            crate::log_vfs!("VFS OPEN /bin/hello: FAIL");

            return;
        }
    };

    let mut buffer = [0u8; 8192];

    let bytes_read = vfs.read(&mut ide, &mut file, &mut buffer);

    crate::log_vfs_raw!("VFS READ bytes: ");

    crate::log_test_raw!("{}", bytes_read as u64);

    crate::log_test_raw!("{}", (b'\n') as char);
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

    crate::log_test!("RAMBLOCK ERROR TEST: PASS");
}

pub fn test_ide() {
    let mut ide = IdeDevice::new();
    let mut buffer = [0u8; 512];

    match ide.read_sector(0, &mut buffer) {
        Ok(()) => {
            crate::log_vfs!("IDE READ: PASS");

            for i in 0..13 {
                crate::log_test_raw!("{}", buffer[i] as char);
            }

            crate::log_test!("");
        }

        Err(_) => {
            crate::log_vfs!("IDE READ: FAIL");
        }
    }
}

pub fn test_vfs_elf() {
    crate::log_vfs!("=========== ELF FROM VFS TEST =========");

    let mut ide = crate::drivers::storage::ide::IdeDevice::new();

    let mut sb_buffer = [0u8; 512];

    if ide.read_sector(2, &mut sb_buffer).is_err() {
        crate::log_vfs!("ELF VFS: SUPERBLOCK READ FAILED");
        return;
    }

    let sb = match crate::filesystem::ext2::parse_superblock(&sb_buffer) {
        Some(sb) => sb,

        None => {
            crate::log_vfs!("ELF VFS: INVALID SUPERBLOCK");
            return;
        }
    };

    let mut gd_buffer = [0u8; 512];

    if ide.read_sector(4, &mut gd_buffer).is_err() {
        crate::log_vfs!("ELF VFS: GROUP DESCRIPTOR READ FAILED");
        return;
    }

    let gd = match crate::filesystem::ext2::parse_group_descriptor(&gd_buffer) {
        Some(gd) => gd,

        None => {
            crate::log_vfs!("ELF VFS: INVALID GROUP DESCRIPTOR");
            return;
        }
    };

    let vfs = crate::filesystem::vfs::Vfs::new(gd.inode_table_block(), 256, sb);

    let mut file = match vfs.open(&mut ide, b"/bin/hello") {
        Some(file) => file,

        None => {
            crate::log_vfs!("ELF VFS: OPEN FAILED");
            return;
        }
    };

    crate::log_vfs_raw!("ELF VFS: inode=");
    crate::log_test_raw!("{}", file.inode_number as u64);
    crate::log_test!("");

    if !file.inode.is_regular_file() {
        crate::log_vfs!("ELF VFS: NOT REGULAR FILE");
        return;
    }

    let file_size = file.inode.size as usize;

    crate::log_vfs_raw!("ELF VFS: file size=");
    crate::log_test_raw!("{}", file_size as u64);
    crate::log_test!("");

    let mut elf_data = alloc::vec![0u8; file_size];

    let bytes_read = vfs.read(&mut ide, &mut file, &mut elf_data);

    crate::log_vfs_raw!("ELF VFS: bytes read=");
    crate::log_test_raw!("{}", bytes_read as u64);
    crate::log_test!("");

    if bytes_read != file_size {
        crate::log_vfs!("ELF VFS: READ FAILED");
        return;
    }

    crate::log_vfs!("ELF VFS: FILE READ OK");

    if elf_data.len() < 4
        || elf_data[0] != 0x7F
        || elf_data[1] != b'E'
        || elf_data[2] != b'L'
        || elf_data[3] != b'F'
    {
        crate::log_vfs!("ELF VFS: BAD MAGIC");
        return;
    }

    crate::log_vfs!("ELF VFS: ELF MAGIC OK");

    let mut allocator = crate::memory::physical::FrameAllocator::new();

    let mut address_space =
        match crate::memory::address_space::AddressSpace::new_user(&mut allocator) {
            Some(space) => space,

            None => {
                crate::log_vfs!("ELF VFS: ADDRESS SPACE FAILED");
                return;
            }
        };

    let loaded =
        match crate::elf::loader::ElfLoader::load(&elf_data, &mut address_space, &mut allocator) {
            Ok(loaded) => loaded,

            Err(error) => {
                crate::log_vfs!("ELF VFS: ELF LOAD FAILED");

                crate::elf::test::print_error(error);
                return;
            }
        };

    crate::log_vfs_raw!("ELF VFS: entry=0x");
    crate::log_test_raw!("{}", loaded.entry);
    crate::log_test!("");

    if loaded.entry != 0x401000 {
        crate::log_vfs!("ELF VFS: ENTRY FAILED");
        return;
    }

    if address_space.translate_page(loaded.entry).is_none() {
        crate::log_vfs!("ELF VFS: ENTRY PAGE FAILED");
        return;
    }

    crate::log_vfs!("ELF VFS: ELF LOAD OK");

    crate::log_vfs!("ELF FROM VFS: PASS");
}
