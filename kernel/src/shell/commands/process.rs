use super::super::parser::trim_spaces;
use crate::drivers::serial;
use crate::shell::state::ShellState;
use crate::task::process_manager;

use super::super::parser::ParseCommand;

const SHELL_CPU: usize = 0;

pub fn wait_for_child(pid: u64) {
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

pub fn exec(command: &ParseCommand, state: &ShellState) {
    if command.argc < 2 {
        serial::println("exec: missing path");
        return;
    }

    let path = command.argv[1];

    if path.is_empty() {
        serial::println("exec: missing path");
        return;
    }

    let mut resolved = [0u8; 256];

    let len = match state.resolve_path(path, &mut resolved) {
        Some(len) => len,
        None => {
            serial::println("exec: path too long");
            return;
        }
    };

    let resolved = &resolved[..len];

    let path_str = match core::str::from_utf8(resolved) {
        Ok(path) => path,
        Err(_) => {
            serial::println("exec: invalid path");
            return;
        }
    };

    serial::println_fmt(core::format_args!("exec: {}", path_str));

    let parent_pid = 0;
    let argv = [path_str];

    match crate::exec::vfs::spawn_exec(resolved, &argv, parent_pid, SHELL_CPU) {
        Ok(result) => wait_for_child(result.process_id),
        Err(_) => {
            serial::println("exec: failed");
        }
    }
}
