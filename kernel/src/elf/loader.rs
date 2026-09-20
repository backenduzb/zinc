use crate::arch::x86_64::paging::PAGE_SIZE;
use crate::memory::address_space::AddressSpace;
use crate::memory::physical::FrameAllocator;

use super::header::{HeaderError, parse_header};
use super::program::{Elf64ProgramHeader, ProgramHeaderError, parse_program_header};
use crate::boot::limine::physical_to_virtual;

const USER_MAX: u64 = 0x0000_8000_0000_0000;

#[derive(Debug)]
pub enum ElfError {
    Header(HeaderError),
    ProgramHeader(ProgramHeaderError),
    InvalidSegment,
    NoLoadSegments,
    InvalidEntry,
    OutOfMemory,
    MappingFailed,
}

pub struct LoadedElf {
    pub entry: u64,
}

pub struct ElfLoader;

impl ElfLoader {
    pub fn load(
        data: &[u8],
        address_space: &mut AddressSpace,
        frame_allocator: &mut FrameAllocator,
    ) -> Result<LoadedElf, ElfError> {
        let header = parse_header(data).map_err(ElfError::Header)?;

        let mut loaded_segments = 0usize;

        for index in 0..header.program_header_count as usize {
            let ph = parse_program_header(data, header.program_header_offset as usize, index)
                .map_err(ElfError::ProgramHeader)?;

            if !ph.is_load() {
                continue;
            }

            Self::load_segment(data, &ph, address_space, frame_allocator)?;

            loaded_segments += 1;
        }

        if loaded_segments == 0 {
            return Err(ElfError::NoLoadSegments);
        }

        if header.entry >= USER_MAX {
            return Err(ElfError::InvalidEntry);
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

        if segment.virtual_address >= USER_MAX || segment_end > USER_MAX {
            return Err(ElfError::InvalidSegment);
        }

        let page_start = align_down(segment.virtual_address);

        let page_end = align_up(segment_end).ok_or(ElfError::InvalidSegment)?;

        let writable = segment.writable();
        let executable = segment.executable();

        let mut virtual_address = page_start;

        while virtual_address < page_end {
            let frame = frame_allocator.alloc_frame().ok_or(ElfError::OutOfMemory)?;

            if !address_space.map_page(
                frame_allocator,
                virtual_address,
                frame.address,
                writable,
                executable,
            ) {
                frame_allocator.free_frame(frame);
                return Err(ElfError::MappingFailed);
            }

            Self::zero_frame(frame.address)?;

            Self::copy_segment_data(data, segment, virtual_address, frame.address)?;

            virtual_address += PAGE_SIZE;
        }

        Ok(())
    }

    fn zero_frame(physical_address: u64) -> Result<(), ElfError> {
        let virtual_address =
            physical_to_virtual(physical_address).ok_or(ElfError::InvalidSegment)?;

        unsafe {
            core::ptr::write_bytes(virtual_address as *mut u8, 0, PAGE_SIZE as usize);
        }

        Ok(())
    }

    fn copy_segment_data(
        data: &[u8],
        segment: &Elf64ProgramHeader,
        page_virtual_address: u64,
        page_physical_address: u64,
    ) -> Result<(), ElfError> {
        if segment.file_size == 0 {
            return Ok(());
        }

        let segment_file_end = segment
            .virtual_address
            .checked_add(segment.file_size)
            .ok_or(ElfError::InvalidSegment)?;

        let page_end = page_virtual_address
            .checked_add(PAGE_SIZE)
            .ok_or(ElfError::InvalidSegment)?;

        let copy_start = core::cmp::max(page_virtual_address, segment.virtual_address);

        let copy_end = core::cmp::min(page_end, segment_file_end);

        if copy_start >= copy_end {
            return Ok(());
        }

        let size = copy_end - copy_start;

        let segment_offset = copy_start - segment.virtual_address;

        let file_offset = segment
            .offset
            .checked_add(segment_offset)
            .ok_or(ElfError::InvalidSegment)?;

        let file_end = file_offset
            .checked_add(size)
            .ok_or(ElfError::InvalidSegment)?;

        if file_end > data.len() as u64 {
            return Err(ElfError::InvalidSegment);
        }

        let source = &data[file_offset as usize..file_end as usize];

        let physical_offset = copy_start - page_virtual_address;

        let destination = physical_to_virtual(page_physical_address + physical_offset)
            .ok_or(ElfError::InvalidSegment)? as *mut u8;

        unsafe {
            core::ptr::copy_nonoverlapping(source.as_ptr(), destination, size as usize);
        }

        Ok(())
    }
}

fn align_down(value: u64) -> u64 {
    value & !(PAGE_SIZE - 1)
}

fn align_up(value: u64) -> Option<u64> {
    value.checked_add(PAGE_SIZE - 1).map(align_down)
}
