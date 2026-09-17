const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;

const EM_X86_64: u16 = 62;

const PT_LOAD: u32 = 1;

const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

const ELF64_HEADER_SIZE: usize = 64;
const ELF64_PROGRAM_HEADER_SIZE: usize = 56;

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
        .checked_add(ELF64_HEADER_SIZE)
        .ok_or(ElfError::InvalidProgramHeaders)? as usize;

    if end > data.len() {
        return Err(ElfError::InvalidProgramHeaders);
    }

    let segment_type = read_u32(data, base).ok_or(ElfError::InvalidSegment)?;
    let flags = read_u32(data, base + 4).ok_or(ElfError::InvalidSegment)?;
    let offset = read_u64(data, base + 8).ok_or(ElfError::InvalidSegment)?;

    let virtual_address = read_u64(data, base + 16).ok_or(ElfError::InvalidSegment)?;

    let file_size = read_u64(data, base + 32).ok_or(ElfError::InvalidSegment)?;

    let memory_size = read_u64(data, base + 40).ok_or(ElfError::InvalidSegment)?;

    let alignment = read_u64(data, base + 48).ok_or(ElfError::InvalidSegment)?;

    if file_size > memory_size {
        return Err(ElfError::InvalidSegment);
    }

    let file_end = offset
        .checked_add(file_size)
        .ok_or(ElfError::InvalidSegment)? as usize;

    if file_end > data.len() {
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
