use crate::drivers::serial;

use super::super::parser::ParseCommand;
use super::super::state::ShellState;

pub fn help() {
    serial::println("commands: help, clear, pwd, echo, ls, cat, touch, write, exec");
}

pub fn clear() {
    serial::print("\x1b[2J\x1b[H");
}

pub fn pwd(state: &ShellState) {
    serial::write_bytes(state.cwd());
    serial::print("\n");
}

pub fn cd(state: &mut ShellState, path: &[u8]) {
    let mut resolved = [0u8; 256];

    let len = match state.resolve_path(path, &mut resolved) {
        Some(len) => len,
        None => {
            serial::println("cd: path too long");
            return;
        }
    };

    let resolved = &resolved[..len];

    let (mut ide, vfs) = match crate::exec::vfs::open_root_vfs() {
        Ok(value) => value,
        Err(_) => {
            serial::println("cd: filesystem unavailable");
            return;
        }
    };

    let file = match vfs.open(&mut ide, resolved) {
        Some(file) => file,
        None => {
            serial::println("cd: no such directory");
            return;
        }
    };

    if !file.inode.is_directory() {
        serial::println("cd: not a directory");
        vfs.close(file);
        return;
    }

    vfs.close(file);

    if !state.set_cwd(resolved) {
        serial::println("cd: failed");
    }
}

pub fn echo(command: &ParseCommand) {
    for i in 1..command.argc {
        if i > 1 {
            serial::putc(b' ');
        }

        serial::write_bytes(command.argv[i]);
    }

    serial::println("");
}
