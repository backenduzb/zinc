use super::header::ELF64_HEADER_SIZE;

pub const ELF64_PROGRAM_HEADER_SIZE: usize = 56;

pub const PT_LOAD: u32 = 1;

pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

#[derive(Clone, Copy)]
pub struct Elf64ProgramHeader {
    pub segment_type: u32,
    pub flags: u32,

    pub offset: u64,
    pub virtual_address: u64,
    pub physical_address: u64,

    pub file_size: u64,
    pub memory_size: u64,

    pub alignment: u64,
}

#[derive(Debug)]
pub enum ProgramHeaderError {
    InvalidIndex,
    InvalidRange,
    InvalidSegment,
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

pub fn parse_program_header(
    data: &[u8],
    offset: usize,
    index: usize,
) -> Result<Elf64ProgramHeader, ProgramHeaderError> {
    let start = offset
        .checked_add(
            index
                .checked_mul(ELF64_PROGRAM_HEADER_SIZE)
                .ok_or(ProgramHeaderError::InvalidRange)?,
        )
        .ok_or(ProgramHeaderError::InvalidRange)?;

    let end = start
        .checked_add(ELF64_PROGRAM_HEADER_SIZE)
        .ok_or(ProgramHeaderError::InvalidRange)?;

    if end > data.len() {
        return Err(ProgramHeaderError::InvalidIndex);
    }

    let segment_type =
        read_u32(data, start).ok_or(ProgramHeaderError::InvalidRange)?;

    let flags =
        read_u32(data, start + 4).ok_or(ProgramHeaderError::InvalidRange)?;

    let file_offset =
        read_u64(data, start + 8).ok_or(ProgramHeaderError::InvalidRange)?;

    let virtual_address =
        read_u64(data, start + 16).ok_or(ProgramHeaderError::InvalidRange)?;

    let physical_address =
        read_u64(data, start + 24).ok_or(ProgramHeaderError::InvalidRange)?;

    let file_size =
        read_u64(data, start + 32).ok_or(ProgramHeaderError::InvalidRange)?;

    let memory_size =
        read_u64(data, start + 40).ok_or(ProgramHeaderError::InvalidRange)?;

    let alignment =
        read_u64(data, start + 48).ok_or(ProgramHeaderError::InvalidRange)?;

    if segment_type == PT_LOAD {
        if memory_size == 0 {
            return Err(ProgramHeaderError::InvalidSegment);
        }

        if file_size > memory_size {
            return Err(ProgramHeaderError::InvalidSegment);
        }

        let file_end = file_offset
            .checked_add(file_size)
            .ok_or(ProgramHeaderError::InvalidSegment)?;

        if file_end > data.len() as u64 {
            return Err(ProgramHeaderError::InvalidSegment);
        }

        if alignment != 0 && !alignment.is_power_of_two() {
            return Err(ProgramHeaderError::InvalidSegment);
        }

        if alignment > 1
            && file_offset % alignment != virtual_address % alignment
        {
            return Err(ProgramHeaderError::InvalidSegment);
        }
    }

    Ok(Elf64ProgramHeader {
        segment_type,
        flags,
        offset: file_offset,
        virtual_address,
        physical_address,
        file_size,
        memory_size,
        alignment,
    })
}

impl Elf64ProgramHeader {
    pub fn is_load(&self) -> bool {
        self.segment_type == PT_LOAD
    }

    pub fn writable(&self) -> bool {
        self.flags & PF_W != 0
    }

    pub fn executable(&self) -> bool {
        self.flags & PF_X != 0
    }

    pub fn readable(&self) -> bool {
        self.flags & PF_R != 0
    }
}