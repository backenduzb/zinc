use super::elf::{ElfLoader, PF_R, ElfError, PF_X};

pub fn test_loader() {
    crate::drivers::serial::println("=== ELF LOADER TEST ===");

    let mut elf = [0u8; 8192];

    elf[0] = 0x7F;
    elf[1] = b'E';
    elf[2] = b'L';
    elf[3] = b'F';

    elf[4] = 2;
    elf[5] = 1;
    elf[6] = 1;

    elf[16..18].copy_from_slice(&2u16.to_le_bytes());
    elf[18..20].copy_from_slice(&62u16.to_le_bytes());
    elf[20..24].copy_from_slice(&1u32.to_le_bytes());
    elf[24..32].copy_from_slice(&0x400000u64.to_le_bytes());

    elf[32..40].copy_from_slice(&64u64.to_le_bytes());
    elf[52..54].copy_from_slice(&64u16.to_le_bytes());
    
    elf[54..56].copy_from_slice(&56u16.to_le_bytes());
    elf[56..58].copy_from_slice(&1u16.to_le_bytes());
    
    let ph = 64usize;

    elf[ph..ph + 4].copy_from_slice(&1u32.to_le_bytes());

    elf[ph + 4..ph + 8].copy_from_slice(
        &(PF_R | PF_X).to_le_bytes()
    );

    elf[ph + 8..ph + 16].copy_from_slice(
        &4096u64.to_le_bytes()
    );
    elf[ph + 16..ph + 24].copy_from_slice(
        &0x400000u64.to_le_bytes()
    );
    elf[ph + 24..ph + 32].copy_from_slice(
        &0u64.to_le_bytes()
    );

    elf[ph + 32..ph + 40].copy_from_slice(
        &1u64.to_le_bytes()
    );
    elf[ph + 40..ph + 48].copy_from_slice(
        &4096u64.to_le_bytes()
    );

    elf[ph + 48..ph + 56].copy_from_slice(
        &4096u64.to_le_bytes()
    );

    elf[4096 - 1] = 0xC3;

    let mut allocator =
        crate::memory::physical::FrameAllocator::new();

    let mut address_space =
        match crate::memory::address_space::AddressSpace::new_user(
            &mut allocator
        ) {
            Some(space) => space,
            None => {
                crate::drivers::serial::println(
                    "ELF loader: AddressSpace FAILED"
                );
                return;
            }
        };

    match ElfLoader::load(
        &elf,
        &mut address_space,
        &mut allocator,
    ) {
        Ok(loaded) => {
            crate::drivers::serial::println("ELF loader: LOAD OK");
    
            crate::drivers::serial::print("ELF entry: 0x");
            crate::drivers::serial::print_u64(loaded.entry);
            crate::drivers::serial::println("");
    
            match address_space.translate_page(0x400000) {
                Some(physical) => {
                    crate::drivers::serial::print(
                        "ELF page mapped: 0x"
                    );
                    crate::drivers::serial::print_u64(physical);
                    crate::drivers::serial::println("");
    
                    crate::drivers::serial::println(
                        "ELF LOADER TEST: PASS"
                    );
                }
    
                None => {
                    crate::drivers::serial::println(
                        "ELF loader: TRANSLATION FAILED"
                    );
                }
            }
        }
    
        Err(error) => {
            match error {
                ElfError::TooSmall => {
                    crate::drivers::serial::println(
                        "ELF ERROR: TooSmall"
                    );
                }
    
                ElfError::BadMagic => {
                    crate::drivers::serial::println(
                        "ELF ERROR: BadMagic"
                    );
                }
    
                ElfError::Not64Bit => {
                    crate::drivers::serial::println(
                        "ELF ERROR: Not64Bit"
                    );
                }
    
                ElfError::NotLittleEndian => {
                    crate::drivers::serial::println(
                        "ELF ERROR: NotLittleEndian"
                    );
                }
    
                ElfError::WrongMachine => {
                    crate::drivers::serial::println(
                        "ELF ERROR: WrongMachine"
                    );
                }
    
                ElfError::InvalidProgramHeaders => {
                    crate::drivers::serial::println(
                        "ELF ERROR: InvalidProgramHeaders"
                    );
                }
    
                ElfError::InvalidSegment => {
                    crate::drivers::serial::println(
                        "ELF ERROR: InvalidSegment"
                    );
                }
            }
    
            crate::drivers::serial::println(
                "ELF LOADER TEST: FAILED"
            );
        }
    }
}