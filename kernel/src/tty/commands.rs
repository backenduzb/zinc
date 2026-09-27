use crate::drivers::serial::{self, println};
use crate::task::process_manager;
use alloc::vec;

const SHELL_CPU: usize = 1;

fn trim_spaces(line: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = line.len();

    while start < end && line[start] == b' ' {
        start += 1;
    }

    while end > start && line[end - 1] == b' ' {
        end -= 1;
    }

    &line[start..end]
}

fn trim_newline(line: &[u8]) -> &[u8] {
    let mut end = line.len();

    while end > 0 && (line[end - 1] == b'\n' || line[end - 1] == b'\r') {
        end -= 1;
    }

    &line[..end]
}

pub fn execute(line: &[u8]) {
    let line = trim_newline(line);

    if line.is_empty() {
        return;
    }

    if line == b"help" {
        serial::println("commands: help, exec <path>, echo <text>");
        return;
    }

    if line == b"clear" {
        serial::print("\x1b[2J\x1b[H");
        return;
    }

    if line.starts_with(b"echo ") {
        serial::println(core::str::from_utf8(&line[5..]).unwrap_or(""));
        return;
    }

    if line.starts_with(b"exec ") {
        exec_command(&line[5..]);
        return;
    }

    if line.starts_with(b"cat ") {
        cat_command(&line[4..]);
        return;
    }

    if line.starts_with(b"ls") {
        if line.contains(&b' ') {
            ls_command(trim_spaces(&line[3..]));
        } else {
            ls_command(b".");
        }
        return;
    }

    if line.starts_with(b"touch ") {
        touch_command(&line[6..]);
        return;
    }

    if line.starts_with(b"write ") {
        write_command(&line[6..]);
        return;
    }

    serial::println("command not found!");
}

fn exec_command(path: &[u8]) {
    let path = trim_spaces(path);

    if path.is_empty() {
        serial::println("failed: exec <path>");
        return;
    }

    let path_str = match core::str::from_utf8(path) {
        Ok(path) => path,
        Err(_) => {
            serial::println("exec: can't run this file");
            return;
        }
    };

    serial::println_fmt(core::format_args!("exec: {}", path_str));

    let parent_pid = 0;

    let argv = [path_str];

    match crate::exec::vfs::spawn_exec(path, &argv, parent_pid, SHELL_CPU) {
        Ok(result) => wait_for_child(result.process_id),
        Err(_) => {
            serial::println("exec: failed");
        }
    }
}

fn write_command(args: &[u8]) {
    let args = trim_spaces(args);

    let separator = match args.iter().position(|&c| c == b' ') {
        Some(pos) => pos,
        None => {
            serial::println("write: usage: write <path> <text>");
            return;
        }
    };

    let path = trim_spaces(&args[..separator]);
    let text = trim_spaces(&args[separator + 1..]);

    if path.is_empty() {
        serial::println("write: missing path");
        return;
    }

    if text.is_empty() {
        serial::println("write: missing text");
        return;
    }

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

    let bytes = vfs.write(&mut ide, &mut file, text);

    vfs.close(file);

    serial::println_fmt(core::format_args!("write: {} bytes", bytes));
}

fn wait_for_child(pid: u64) {
    for _ in 0..200 {
        let done = unsafe {
            let pm = &mut *process_manager();

            match pm.get(pid) {
                Some(process) => {
                    process.state() == crate::task::process::ProcessState::Terminated
                        && !pm.has_live_threads(pid)
                }
                None => true,
            }
        };

        if done {
            unsafe {
                let pm = &mut *process_manager();
                let mut allocator = crate::memory::physical::FrameAllocator::new();

                let _ = pm.reap(pid, &mut allocator);
            }

            return;
        }

        crate::task::sleep(1);
    }

    serial::println("exec: wait timeout");
}

fn cat_command(path: &[u8]) {
    let path = trim_spaces(path);

    if path.is_empty() {
        serial::println("cat: missing file");
        return;
    }

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
    let mut data = alloc::vec![0u8; size];

    let bytes_read = vfs.read(&mut ide, &mut file, &mut data);

    serial::write_bytes(&data[..bytes_read]);
    println("");

    vfs.close(file);
}

fn ls_command(path: &[u8]) {
    let path = trim_spaces(path);

    let path = if path.is_empty() { b"/" } else { path };

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

    let size = file.inode.size as usize;
    let mut data = vec![0u8; size];

    let read = vfs.read(&mut ide, &mut file, &mut data);

    crate::filesystem::directory::list_entries(&data, read, |inode, name, file_type| {
        let name = core::str::from_utf8(name).unwrap_or("?");

        serial::println_fmt(core::format_args!("{}  {}  {}", inode, file_type, name));
    });

    vfs.close(file);
}

fn touch_command(path: &[u8]) {
    let path = trim_spaces(path);

    if path.is_empty() {
        serial::println("touch: missing file");
        return;
    }

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
