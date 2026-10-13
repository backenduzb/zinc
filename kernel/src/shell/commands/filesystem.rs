use super::super::parser::{ParseCommand, trim_spaces};
use super::super::state::ShellState;

use crate::drivers::serial;
use alloc::vec;

fn resolve_path<'a>(
    state: &ShellState,
    path: &[u8],
    buffer: &'a mut [u8; 256],
) -> Option<&'a [u8]> {
    let len = state.resolve_path(path, buffer)?;
    Some(&buffer[..len])
}

pub fn write(command: &ParseCommand, state: &ShellState) {
    if command.argc < 3 {
        serial::println("write: usage: write <path> <text>");
        return;
    }

    let path = trim_spaces(command.argv[1]);

    if path.is_empty() {
        serial::println("write: missing path");
        return;
    }

    let mut text = [0u8; 1024];
    let mut text_len = 0;

    for i in 2..command.argc {
        let arg = command.argv[i];

        if text_len > 0 {
            if text_len >= text.len() {
                serial::println("write: text too long");
                return;
            }

            text[text_len] = b' ';
            text_len += 1;
        }

        if text_len + arg.len() > text.len() {
            serial::println("write: text too long");
            return;
        }

        text[text_len..text_len + arg.len()].copy_from_slice(arg);
        text_len += arg.len();
    }

    if text_len == 0 {
        serial::println("write: missing text");
        return;
    }

    let mut resolved_buffer = [0u8; 256];

    let path = match resolve_path(state, path, &mut resolved_buffer) {
        Some(path) => path,
        None => {
            serial::println("write: path too long");
            return;
        }
    };

    let (mut ide, vfs) = match crate::exec::vfs::open_root_vfs() {
        Ok(value) => value,
        Err(_) => {
            serial::println("write: filesystem unavailable");
            return;
        }
    };

    let mut file = match vfs.open(&mut ide, path) {
        Some(file) => file,
        None => {
            serial::println("write: file not found");
            return;
        }
    };

    if !file.inode.is_regular_file() {
        serial::println("write: not a regular file");
        vfs.close(file);
        return;
    }

    let bytes = vfs.write(&mut ide, &mut file, &text[..text_len]);

    vfs.close(file);

    serial::println_fmt(core::format_args!("write: {} bytes", bytes));
}

pub fn cat(command: &ParseCommand, state: &ShellState) {
    if command.argc < 2 {
        serial::println("cat: missing file path");
        return;
    }

    let path = trim_spaces(command.argv[1]);

    if path.is_empty() {
        serial::println("cat: missing file");
        return;
    }

    let mut resolved_buffer = [0u8; 256];

    let path = match resolve_path(state, path, &mut resolved_buffer) {
        Some(path) => path,
        None => {
            serial::println("cat: path too long");
            return;
        }
    };

    let (mut ide, vfs) = match crate::exec::vfs::open_root_vfs() {
        Ok(value) => value,
        Err(_) => {
            serial::println("cat: filesystem unavailable");
            return;
        }
    };

    let mut file = match vfs.open(&mut ide, path) {
        Some(file) => file,
        None => {
            serial::println("cat: file not found");
            return;
        }
    };

    if !file.inode.is_regular_file() {
        serial::println("cat: not a regular file");
        vfs.close(file);
        return;
    }

    let size = file.inode.size as usize;
    let mut data = vec![0u8; size];

    let bytes_read = vfs.read(&mut ide, &mut file, &mut data);

    serial::write_bytes(&data[..bytes_read]);
    serial::println("");

    vfs.close(file);
}

pub fn ls(command: &ParseCommand, state: &ShellState) {
    let path = if command.argc >= 2 {
        trim_spaces(command.argv[1])
    } else {
        b""
    };

    let path = if path.is_empty() { b"." } else { path };

    let mut resolved_buffer = [0u8; 256];

    let path = match resolve_path(state, path, &mut resolved_buffer) {
        Some(path) => path,
        None => {
            serial::println("ls: path too long");
            return;
        }
    };

    let (mut ide, vfs) = match crate::exec::vfs::open_root_vfs() {
        Ok(value) => value,
        Err(_) => {
            serial::println("ls: filesystem unavailable");
            return;
        }
    };

    let mut file = match vfs.open(&mut ide, path) {
        Some(file) => file,
        None => {
            serial::println("ls: directory not found");
            return;
        }
    };

    if !file.inode.is_directory() {
        serial::println("ls: not a directory");
        vfs.close(file);
        return;
    }

    let size = file.inode.size as usize;
    let mut data = vec![0u8; size];

    let read = vfs.read(&mut ide, &mut file, &mut data);

    crate::filesystem::directory::list_entries(&data, read, |inode, name, file_type| {
        let name = core::str::from_utf8(name).unwrap_or("?");

        serial::println_fmt(core::format_args!("{}  {}  {}", inode, file_type, name));
    });

    vfs.close(file);
}

pub fn touch(command: &ParseCommand, state: &ShellState) {
    if command.argc < 2 {
        serial::println("touch: missing file path");
        return;
    }

    let path = trim_spaces(command.argv[1]);

    if path.is_empty() {
        serial::println("touch: missing file");
        return;
    }

    let mut resolved_buffer = [0u8; 256];

    let path = match resolve_path(state, path, &mut resolved_buffer) {
        Some(path) => path,
        None => {
            serial::println("touch: path too long");
            return;
        }
    };

    let slash = match path.iter().rposition(|&c| c == b'/') {
        Some(pos) => pos,
        None => {
            serial::println("touch: invalid path");
            return;
        }
    };

    let parent = if slash == 0 { b"/" } else { &path[..slash] };

    let name = &path[slash + 1..];

    if name.is_empty() {
        serial::println("touch: invalid filename");
        return;
    }

    let (mut ide, vfs) = match crate::exec::vfs::open_root_vfs() {
        Ok(value) => value,
        Err(_) => {
            serial::println("touch: filesystem unavailable");
            return;
        }
    };

    match vfs.create(&mut ide, parent, name) {
        Ok(inode) => {
            serial::println_fmt(core::format_args!(
                "touch: created {} inode={}",
                core::str::from_utf8(path).unwrap_or("?"),
                inode
            ));
        }

        Err(error) => {
            serial::println_fmt(core::format_args!("touch: {}", error));
        }
    }
}
