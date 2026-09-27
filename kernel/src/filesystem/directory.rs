#[derive(Clone, Copy)]
pub struct Ext2DirEntryHeader {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
}

fn read_u16(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

fn read_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

pub fn parse_entry(buf: &[u8]) -> Option<Ext2DirEntryHeader> {
    if buf.len() < 8 {
        return None;
    }

    let inode = read_u32(buf, 0);
    let rec_len = read_u16(buf, 4);
    let name_len = buf[6];
    let file_type = buf[7];

    if rec_len < 8 {
        return None;
    }

    if rec_len as usize > buf.len() {
        return None;
    }

    if 8 + name_len as usize > rec_len as usize {
        return None;
    }

    Some(Ext2DirEntryHeader {
        inode,
        rec_len,
        name_len,
        file_type,
    })
}

pub fn entry_name<'a>(buf: &'a [u8], entry: &Ext2DirEntryHeader) -> Option<&'a [u8]> {
    let start = 8;
    let end = start + entry.name_len as usize;

    if end > buf.len() {
        return None;
    }

    Some(&buf[start..end])
}

pub fn lookup<'a>(buf: &'a [u8], directory_size: usize, name: &[u8]) -> Option<u32> {
    let mut offset = 0usize;

    while offset < directory_size {
        let current = &buf[offset..];

        let entry = parse_entry(current)?;

        if entry.inode != 0 {
            if let Some(entry_name) = entry_name(current, &entry) {
                if entry_name == name {
                    return Some(entry.inode);
                }
            }
        }

        if entry.rec_len == 0 {
            return None;
        }

        offset += entry.rec_len as usize;
    }

    None
}

pub fn list_entries<F>(buf: &[u8], directory_size: usize, mut callback: F)
where
    F: FnMut(u32, &[u8], u8),
{
    let mut offset = 0usize;
    let limit = core::cmp::min(directory_size, buf.len());

    while offset < limit {
        if offset + 8 > limit {
            break;
        }

        let current = &buf[offset..limit];

        let Some(entry) = parse_entry(current) else {
            break;
        };

        if entry.inode != 0 {
            if let Some(name) = entry_name(current, &entry) {
                callback(entry.inode, name, entry.file_type);
            }
        }

        let rec_len = entry.rec_len as usize;

        if rec_len == 0 || offset + rec_len > limit {
            break;
        }

        offset += rec_len;
    }
}

pub fn add_entry(
    buf: &mut [u8],
    directory_size: usize,
    inode: u32,
    name: &[u8],
    file_type: u8,
) -> bool {
    if inode == 0 || name.is_empty() || name.len() > 255 {
        return false;
    }

    let needed = (8 + name.len() + 3) & !3;

    if needed > directory_size || directory_size > buf.len() {
        return false;
    }

    let mut offset = 0usize;

    while offset < directory_size {
        let current = &buf[offset..directory_size];

        let Some(entry) = parse_entry(current) else {
            return false;
        };

        let rec_len = entry.rec_len as usize;

        if rec_len == 0 || offset + rec_len > directory_size {
            return false;
        }

        let minimal_current = (8 + entry.name_len as usize + 3) & !3;

        let slack = rec_len.saturating_sub(minimal_current);

        if slack >= needed {
            let new_offset = offset + minimal_current;

            buf[offset + 4..offset + 6].copy_from_slice(&(minimal_current as u16).to_le_bytes());

            let new_rec_len = rec_len - minimal_current;

            buf[new_offset..new_offset + 4].copy_from_slice(&inode.to_le_bytes());

            buf[new_offset + 4..new_offset + 6]
                .copy_from_slice(&(new_rec_len as u16).to_le_bytes());

            buf[new_offset + 6] = name.len() as u8;
            buf[new_offset + 7] = file_type;

            let name_start = new_offset + 8;
            let name_end = name_start + name.len();

            buf[name_start..name_end].copy_from_slice(name);

            let end = new_offset + new_rec_len;
            if name_end < end {
                buf[name_end..end].fill(0);
            }

            return true;
        }

        offset += rec_len;
    }

    false
}
