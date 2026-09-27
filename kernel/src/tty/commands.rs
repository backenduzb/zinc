use crate::drivers::serial;
use crate::task::process_manager;

const SHELL_CPU: usize = 1;

fn trim_spaces(line: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = line.len();

    while start < end && line[start] == b' ' {
        start += 1;
    }

    while end > start && line[end] == b' ' {
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

    let parent_pid = unsafe {
        let pm = process_manager();
        let pm = &mut *pm;

        match pm.find_live_child(0) {
            Some(pid) => pid,
            None => {
                serial::println("exec: no shell process");
                return;
            }
        }
    };

    let argv = [path_str];

    match crate::exec::vfs::spawn_exec(path, &argv, parent_pid, SHELL_CPU) {
        Ok(result) => wait_for_child(result.process_id),
        Err(_) => {
            serial::println("exec: failed");
        }
    }
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
