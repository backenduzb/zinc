use super::header::HeaderError;
use super::loader::ElfError;
use super::loader::ElfLoader;
use super::program::{PF_R, PF_X};
use alloc::vec;

const TEST_ENTRY: u64 = 0x0040_0000;
const TEST_OFFSET: u64 = 0x1000;
const TEST_FILE_SIZE: u64 = 16;
const TEST_MEMORY_SIZE: u64 = 0x1000;

pub fn test_loader() {
    crate::log_exec!("=== ELF LOADER TEST ===");

    let mut elf = [0u8; 8192];

    elf[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    elf[4] = 2;
    elf[5] = 1;
    elf[6] = 1;

    elf[16..18].copy_from_slice(&2u16.to_le_bytes());
    elf[18..20].copy_from_slice(&62u16.to_le_bytes());
    elf[20..24].copy_from_slice(&1u32.to_le_bytes());
    elf[24..32].copy_from_slice(&TEST_ENTRY.to_le_bytes());

    elf[32..40].copy_from_slice(&64u64.to_le_bytes());
    elf[52..54].copy_from_slice(&64u16.to_le_bytes());
    elf[54..56].copy_from_slice(&56u16.to_le_bytes());
    elf[56..58].copy_from_slice(&1u16.to_le_bytes());

    let ph = 64usize;

    elf[ph..ph + 4].copy_from_slice(&1u32.to_le_bytes());
    elf[ph + 4..ph + 8].copy_from_slice(&(PF_R | PF_X).to_le_bytes());

    elf[ph + 8..ph + 16].copy_from_slice(&TEST_OFFSET.to_le_bytes());
    elf[ph + 16..ph + 24].copy_from_slice(&TEST_ENTRY.to_le_bytes());
    elf[ph + 24..ph + 32].copy_from_slice(&0u64.to_le_bytes());

    elf[ph + 32..ph + 40].copy_from_slice(&TEST_FILE_SIZE.to_le_bytes());
    elf[ph + 40..ph + 48].copy_from_slice(&TEST_MEMORY_SIZE.to_le_bytes());
    elf[ph + 48..ph + 56].copy_from_slice(&0x1000u64.to_le_bytes());

    let payload = [
        0x48, 0x31, 0xC0, 0x48, 0x31, 0xFF, 0x48, 0x31, 0xF6, 0x48, 0x31, 0xD2, 0xC3, 0x90, 0x90,
        0x90,
    ];

    let payload_start = TEST_OFFSET as usize;
    let payload_end = payload_start + payload.len();

    elf[payload_start..payload_end].copy_from_slice(&payload);

    crate::log_exec!("ELF test image created");

    let mut allocator = crate::memory::physical::FrameAllocator::new();

    let mut address_space =
        match crate::memory::address_space::AddressSpace::new_user(&mut allocator) {
            Some(space) => space,
            None => {
                crate::log_exec!("ELF TEST: AddressSpace creation FAILED");
                return;
            }
        };

    crate::log_exec!("ELF TEST: AddressSpace created");

    let loaded = match ElfLoader::load(&elf, &mut address_space, &mut allocator) {
        Ok(loaded) => {
            crate::log_exec!("ELF TEST: LOAD OK");

            loaded
        }

        Err(error) => {
            print_error(error);
            crate::log_exec!("ELF LOADER TEST: FAILED");
            return;
        }
    };

    crate::log_exec_raw!("ELF entry: 0x");
    crate::log_exec_raw!("{}", loaded.entry);
    crate::log_exec!("");

    if loaded.entry != TEST_ENTRY {
        crate::log_exec!("ELF TEST: ENTRY FAILED");
        return;
    }

    crate::log_exec!("ELF TEST: entry OK");

    let physical = match address_space.translate_page(TEST_ENTRY) {
        Some(physical) => physical,

        None => {
            crate::log_exec!("ELF TEST: PAGE TRANSLATION FAILED");
            return;
        }
    };

    crate::log_exec_raw!("ELF mapped physical page: 0x");
    crate::log_exec_raw!("{}", physical);
    crate::log_exec!("");

    match address_space.probe_page(TEST_ENTRY) {
        Some((user, writable, executable)) => {
            crate::log_exec_raw!("ELF permissions: U=");
            crate::log_exec_raw!("{}", user as u64);

            crate::log_exec_raw!(" W=");
            crate::log_exec_raw!("{}", writable as u64);

            crate::log_exec_raw!(" X=");
            crate::log_exec_raw!("{}", executable as u64);

            crate::log_exec!("");

            if !user || writable || !executable {
                crate::log_exec!("ELF TEST: PERMISSIONS FAILED");
                return;
            }
        }

        None => {
            crate::log_exec!("ELF TEST: PERMISSION PROBE FAILED");
            return;
        }
    }

    let physical_virtual = match crate::boot::limine::physical_to_virtual(physical) {
        Some(address) => address,

        None => {
            crate::log_exec!("ELF TEST: PHYS->VIRT FAILED");
            return;
        }
    };

    unsafe {
        let loaded_bytes =
            core::slice::from_raw_parts(physical_virtual as *const u8, TEST_MEMORY_SIZE as usize);

        if loaded_bytes[..payload.len()] != payload {
            crate::log_exec!("ELF TEST: DATA COPY FAILED");
            return;
        }

        crate::log_exec!("ELF TEST: data copy OK");

        let bss_start = payload.len();

        if loaded_bytes[bss_start..].iter().any(|&byte| byte != 0) {
            crate::log_exec!("ELF TEST: BSS ZERO FAILED");
            return;
        }

        crate::log_exec!("ELF TEST: BSS zero OK");
    }

    crate::log_exec!("ELF LOADER TEST: PASS");
}

pub fn print_error(error: ElfError) {
    crate::log_exec!("ELF TEST: LOAD FAILED");

    match error {
        ElfError::Header(error) => {
            crate::log_exec_raw!("ELF header error: ");

            match error {
                HeaderError::TooSmall => {
                    crate::log_exec!("TooSmall");
                }
                HeaderError::BadMagic => {
                    crate::log_exec!("BadMagic");
                }
                HeaderError::Not64Bit => {
                    crate::log_exec!("Not64Bit");
                }
                HeaderError::NotLittleEndian => {
                    crate::log_exec!("NotLittleEndian");
                }
                HeaderError::WrongType => {
                    crate::log_exec!("WrongType");
                }
                HeaderError::WrongMachine => {
                    crate::log_exec!("WrongMachine");
                }
                HeaderError::InvalidVersion => {
                    crate::log_exec!("InvalidVersion");
                }
                HeaderError::InvalidHeaderSize => {
                    crate::log_exec!("InvalidHeaderSize");
                }
                HeaderError::InvalidProgramHeaders => {
                    crate::log_exec!("InvalidProgramHeaders");
                }
            }
        }

        ElfError::ProgramHeader(_) => {
            crate::log_exec!("ELF program header error");
        }

        ElfError::InvalidSegment => {
            crate::log_exec!("ELF InvalidSegment");
        }

        ElfError::NoLoadSegments => {
            crate::log_exec!("ELF NoLoadSegments");
        }

        ElfError::InvalidEntry => {
            crate::log_exec!("ELF InvalidEntry");
        }

        ElfError::OutOfMemory => {
            crate::log_exec!("ELF OutOfMemory");
        }

        ElfError::MappingFailed => {
            crate::log_exec!("ELF MappingFailed");
        }
    }
}

