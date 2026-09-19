use super::program::ELF64_PROGRAM_HEADER_SIZE;

pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
pub const ELFCLASS64: u8 = 2;
pub const ELFDATA2LSB: u8 = 1;

pub const EM_X86_64: u16 = 62;
pub const ET_EXEC: u16 = 2;
pub const EV_CURRENT: u32 = 1;

pub const ELF64_HEADER_SIZE: usize = 64;

#[derive(Clone, Copy)]
pub struct Elf64Header {
    pub object_type: u16,
    pub machine: u16,
    pub version: u32,

    pub entry: u64,
    pub program_header_offset: u64,

    pub header_size: u16,
    pub program_header_entry_size: u16,
    pub program_header_count: u16,
}

#[derive(Debug)]
pub enum HeaderError {
    TooSmall,
    BadMagic,
    Not64Bit,
    NotLittleEndian,
    WrongType,
    WrongMachine,
    InvalidVersion,
    InvalidHeaderSize,
    InvalidProgramHeaders,
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
    ]))
}

fn read_u64(data: &[u8], offset: usize) -> Option<u64> {
    let bytes = data.get(offset..offset + 8)?;

    Some(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

pub fn parse_header(data: &[u8]) -> Result<Elf64Header, HeaderError> {
    if data.len() < ELF64_HEADER_SIZE {
        return Err(HeaderError::TooSmall);
    }

    if data[0..4] != ELF_MAGIC {
        return Err(HeaderError::BadMagic);
    }

    if data[4] != ELFCLASS64 {
        return Err(HeaderError::Not64Bit);
    }

    if data[5] != ELFDATA2LSB {
        return Err(HeaderError::NotLittleEndian);
    }

    let object_type =
        read_u16(data, 16).ok_or(HeaderError::TooSmall)?;

    if object_type != ET_EXEC {
        return Err(HeaderError::WrongType);
    }

    let machine =
        read_u16(data, 18).ok_or(HeaderError::TooSmall)?;

    if machine != EM_X86_64 {
        return Err(HeaderError::WrongMachine);
    }

    let version =
        read_u32(data, 20).ok_or(HeaderError::TooSmall)?;

    if version != EV_CURRENT {
        return Err(HeaderError::InvalidVersion);
    }

    let entry =
        read_u64(data, 24).ok_or(HeaderError::TooSmall)?;

    let program_header_offset =
        read_u64(data, 32).ok_or(HeaderError::TooSmall)?;

    let header_size =
        read_u16(data, 52).ok_or(HeaderError::TooSmall)?;

    if header_size as usize != ELF64_HEADER_SIZE {
        return Err(HeaderError::InvalidHeaderSize);
    }

    let program_header_entry_size =
        read_u16(data, 54).ok_or(HeaderError::TooSmall)?;

    if program_header_entry_size as usize != ELF64_PROGRAM_HEADER_SIZE {
        return Err(HeaderError::InvalidProgramHeaders);
    }

    let program_header_count =
        read_u16(data, 56).ok_or(HeaderError::TooSmall)?;

    let phoff = usize::try_from(program_header_offset)
        .map_err(|_| HeaderError::InvalidProgramHeaders)?;

    let phsize = (program_header_entry_size as usize)
        .checked_mul(program_header_count as usize)
        .ok_or(HeaderError::InvalidProgramHeaders)?;

    let phend = phoff
        .checked_add(phsize)
        .ok_or(HeaderError::InvalidProgramHeaders)?;

    if phend > data.len() {
        return Err(HeaderError::InvalidProgramHeaders);
    }

    Ok(Elf64Header {
        object_type,
        machine,
        version,
        entry,
        program_header_offset,
        header_size,
        program_header_entry_size,
        program_header_count,
    })
}