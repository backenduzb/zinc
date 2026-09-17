use alloc::collections::btree_map::Values;

use crate::arch::x86_64::paging::PAGE_SIZE;
use crate::memory::address_space::AddressSpace;
use crate::memory::physical::FrameAllocator;

const USER_MAX: u64 = 0x0000_8000_0000_0000;

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;

const EM_X86_64: u16 = 62;

pub const PT_LOAD: u32 = 1;

pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

const ELF64_HEADER_SIZE: usize = 64;
const ELF64_PROGRAM_HEADER_SIZE: usize = 56;

pub struct ElfLoader;

pub struct LoadedElf {
    pub entry: u64,
}

#[derive(Clone, Copy)]
pub struct Elf64Header {
    pub entry: u64,
    pub program_header_offset: u64,
    pub program_header_entry_size: u16,
    pub program_header_count: u16,
}

#[derive(Clone, Copy)]
pub struct Elf64ProgramHeader {
    pub segment_type: u32,
    pub flags: u32,
    pub offset: u64,
    pub virtual_address: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub alignment: u64,
}

pub enum ElfError {
    TooSmall,
    BadMagic,
    Not64Bit,
    NotLittleEndian,
    WrongMachine,
    InvalidProgramHeaders,
    InvalidSegment,
}

impl ElfLoader {
    pub fn load(
        data: &[u8],
        address_space: &mut AddressSpace,
        frame_allocator: &mut FrameAllocator,
    ) -> Result<LoadedElf, ElfError> {
        let header = parse_header(data)?;

        let mut loaded_segments = 0usize;

        for index in 0..header.program_header_count as usize {
            let segment = parse_program_header(data, &header, index)?;

            if segment.segment_type != PT_LOAD {
                continue;
            }

            validate_load_segment(data, &segment)?;

            Self::load_segment(data, &segment, address_space, frame_allocator)?;

            loaded_segments += 1;
        }

        if loaded_segments == 0 {
            return Err(ElfError::InvalidSegment);
        }

        if header.entry >= USER_MAX {
            return Err(ElfError::InvalidSegment);
        }

        Ok(LoadedElf {
            entry: header.entry,
        })
    }

    fn load_segment(
        data: &[u8],
        segment: &Elf64ProgramHeader,
        address_space: &mut AddressSpace,
        frame_allocator: &mut FrameAllocator,
    ) -> Result<(), ElfError> {
        let segment_end = segment
            .virtual_address
            .checked_add(segment.memory_size)
            .ok_or(ElfError::InvalidSegment)?;

        if segment_end > USER_MAX {
            return Err(ElfError::InvalidSegment);
        }

        let page_start = align_down(segment.virtual_address);

        let page_end = align_up(segment_end).ok_or(ElfError::InvalidSegment)?;

        let writable = segment.flags & PF_W != 0;
        let executable = segment.flags & PF_X != 0;

        let mut page = page_start;

        while page < page_end {
            crate::drivers::serial::print("ELF: allocating page 0x");
            crate::drivers::serial::print_u64(page);
            crate::drivers::serial::println("");

            let frame = frame_allocator
                .alloc_frame()
                .ok_or(ElfError::InvalidSegment)?;

            crate::drivers::serial::print("ELF: frame=0x");
            crate::drivers::serial::print_u64(frame.address);
            crate::drivers::serial::println("");

            if !address_space.map_page(frame_allocator, page, frame.address, writable, executable) {
                crate::drivers::serial::println("ELF: MAP_PAGE FAILED");

                frame_allocator.free_frame(frame);

                return Err(ElfError::InvalidSegment);
            }

            crate::drivers::serial::println("ELF: page mapped");

            let physical = crate::boot::limine::physical_to_virtual(frame.address)
                .ok_or(ElfError::InvalidSegment)?;

            unsafe {
                core::ptr::write_bytes(physical as *mut u8, 0, PAGE_SIZE as usize);
            }

            Self::copy_segment_data(data, segment, page, physical)?;

            crate::drivers::serial::println("ELF: page loaded");

            page = page
                .checked_add(PAGE_SIZE)
                .ok_or(ElfError::InvalidSegment)?;
        }

        Ok(())
    }

    fn copy_segment_data(
        data: &[u8],
        segment: &Elf64ProgramHeader,
        page_virtual: u64,
        page_physical_virtual: u64,
    ) -> Result<(), ElfError> {
        let segment_file_end = segment
            .offset
            .checked_add(segment.file_size)
            .ok_or(ElfError::InvalidSegment)?;

        if segment_file_end > data.len() as u64 {
            return Err(ElfError::InvalidSegment);
        }

        let page_end = page_virtual
            .checked_add(PAGE_SIZE)
            .ok_or(ElfError::InvalidSegment)?;

        let file_start_va = segment.virtual_address;
        let file_end_va = segment
            .virtual_address
            .checked_add(segment.file_size)
            .ok_or(ElfError::InvalidSegment)?;

        let copy_start = core::cmp::max(page_virtual, file_start_va);
        let copy_end = core::cmp::min(page_end, file_end_va);

        if copy_start >= copy_end {
            return Ok(());
        }

        let destination_offset = (copy_start - page_virtual) as usize;

        let source_offset = (segment.offset + (copy_start - segment.virtual_address)) as usize;

        let length = (copy_end - copy_start) as usize;

        unsafe {
            core::ptr::copy_nonoverlapping(
                data.as_ptr().add(source_offset),
                (page_physical_virtual as *mut u8).add(destination_offset),
                length,
            );
        }

        Ok(())
    }
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;

    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;

    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    let bytes = data.get(offset..offset + 8)?;

    Some(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

pub fn parse_header(data: &[u8]) -> Result<Elf64Header, ElfError> {
    if data.len() < ELF64_HEADER_SIZE {
        return Err(ElfError::TooSmall);
    }

    if data[0..4] != ELF_MAGIC {
        return Err(ElfError::BadMagic);
    }

    if data[4] != ELFCLASS64 {
        return Err(ElfError::Not64Bit);
    }

    if data[5] != ELFDATA2LSB {
        return Err(ElfError::NotLittleEndian);
    }

    let machine = read_u16(data, 18).ok_or(ElfError::TooSmall)?;

    if machine != EM_X86_64 {
        return Err(ElfError::WrongMachine);
    }

    let entry = read_u64(data, 24).ok_or(ElfError::TooSmall)?;

    let program_header_offset = read_u64(data, 32).ok_or(ElfError::TooSmall)?;

    let program_header_entry_size = read_u16(data, 54).ok_or(ElfError::TooSmall)?;

    let program_header_count = read_u16(data, 56).ok_or(ElfError::TooSmall)?;

    if program_header_entry_size as usize != ELF64_PROGRAM_HEADER_SIZE {
        return Err(ElfError::InvalidProgramHeaders);
    }

    let ph_offset = program_header_offset as usize;

    let ph_size = ELF64_PROGRAM_HEADER_SIZE
        .checked_mul(program_header_count as usize)
        .ok_or(ElfError::InvalidProgramHeaders)?;

    let ph_end = ph_offset
        .checked_add(ph_size)
        .ok_or(ElfError::InvalidProgramHeaders)?;

    if ph_end > data.len() {
        return Err(ElfError::InvalidProgramHeaders);
    }

    Ok(Elf64Header {
        entry,
        program_header_offset,
        program_header_entry_size,
        program_header_count,
    })
}

pub fn parse_program_header(
    data: &[u8],
    header: &Elf64Header,
    index: usize,
) -> Result<Elf64ProgramHeader, ElfError> {
    if index >= header.program_header_count as usize {
        return Err(ElfError::InvalidProgramHeaders);
    }

    let base = header
        .program_header_offset
        .checked_add(
            (index as u64)
                .checked_mul(header.program_header_entry_size as u64)
                .ok_or(ElfError::InvalidProgramHeaders)?,
        )
        .ok_or(ElfError::InvalidProgramHeaders)? as usize;

    let end = base
        .checked_add(ELF64_PROGRAM_HEADER_SIZE)
        .ok_or(ElfError::InvalidProgramHeaders)?;

    if end > data.len() {
        return Err(ElfError::InvalidProgramHeaders);
    }

    let segment_type = read_u32(data, base).ok_or(ElfError::InvalidProgramHeaders)?;

    let flags = read_u32(data, base + 4).ok_or(ElfError::InvalidProgramHeaders)?;

    let offset = read_u64(data, base + 8).ok_or(ElfError::InvalidProgramHeaders)?;

    let virtual_address = read_u64(data, base + 16).ok_or(ElfError::InvalidProgramHeaders)?;

    let file_size = read_u64(data, base + 32).ok_or(ElfError::InvalidSegment)?;

    let memory_size = read_u64(data, base + 40).ok_or(ElfError::InvalidProgramHeaders)?;

    let alignment = read_u64(data, base + 48).ok_or(ElfError::InvalidProgramHeaders)?;

    crate::drivers::serial::println("ELF PH:");

    crate::drivers::serial::print("  type=");
    crate::drivers::serial::print_u64(segment_type as u64);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  flags=");
    crate::drivers::serial::print_u64(flags as u64);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  offset=");
    crate::drivers::serial::print_u64(offset);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  vaddr=0x");
    crate::drivers::serial::print_u64(virtual_address);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  filesz=");
    crate::drivers::serial::print_u64(file_size);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  memsz=");
    crate::drivers::serial::print_u64(memory_size);
    crate::drivers::serial::println("");

    crate::drivers::serial::print("  align=");
    crate::drivers::serial::print_u64(alignment);
    crate::drivers::serial::println("");

    if file_size > memory_size {
        crate::drivers::serial::println("ELF PH: filesz > memsz");
        return Err(ElfError::InvalidSegment);
    }

    let file_end = offset
        .checked_add(file_size)
        .ok_or(ElfError::InvalidSegment)?;

    if file_end > data.len() as u64 {
        crate::drivers::serial::println("ELF PH: file outside ELF");
        return Err(ElfError::InvalidSegment);
    }

    Ok(Elf64ProgramHeader {
        segment_type,
        flags,
        offset,
        virtual_address,
        file_size,
        memory_size,
        alignment,
    })
}

pub fn validate_load_segment(data: &[u8], segment: &Elf64ProgramHeader) -> Result<(), ElfError> {
    if segment.segment_type != PT_LOAD {
        return Ok(());
    }

    if segment.memory_size == 0 {
        return Err(ElfError::InvalidSegment);
    }

    if segment.file_size > segment.memory_size {
        return Err(ElfError::InvalidSegment);
    }

    let file_end = segment
        .offset
        .checked_add(segment.file_size)
        .ok_or(ElfError::InvalidSegment)?;
    if file_end > data.len() as u64 {
        return Err(ElfError::InvalidSegment);
    }

    let virtual_end = segment
        .virtual_address
        .checked_add(segment.memory_size)
        .ok_or(ElfError::InvalidSegment)?;

    const USER_MAX: u64 = 0x0000_8000_0000_0000;

    if virtual_end > USER_MAX {
        return Err(ElfError::InvalidSegment);
    }

    if segment.alignment != 0 && (segment.alignment & (segment.alignment - 1)) != 0 {
        return Err(ElfError::InvalidSegment);
    }

    Ok(())
}

pub fn align_down(value: u64) -> u64 {
    value & !(PAGE_SIZE - 1)
}

pub fn align_up(value: u64) -> Option<u64> {
    value
        .checked_add(PAGE_SIZE - 1)
        .map(|value| value & !(PAGE_SIZE - 1))
}
