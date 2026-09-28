pub mod parser;
pub mod state;
pub mod commands;

use parser::parse;
use state::ShellState;


pub struct Shell {
    state: ShellState,
}

static mut SHELL: Option<Shell> = None;


impl Shell {
    pub const fn new() -> Self {
        Self {
            state: ShellState::new(),
        }
    }

    pub fn execute(&mut self, line: &[u8]) {
        let command = parse(line);

        if command.argc == 0 {
            return;
        }

        match command.command() {
            Some(b"pwd") => {
                crate::drivers::serial::write_bytes(self.state.cwd());
                crate::drivers::serial::println("");
            }

            Some(b"echo") => {
                for i in 1..command.argc {
                    if i > 1 {
                        crate::drivers::serial::putc(b' ');
                    }

                    crate::drivers::serial::write_bytes(command.argv[i]);
                }
                crate::drivers::serial::println("");
            }

            _ => {
                crate::drivers::serial::println("command not found");
            }
        }
    }
}

pub fn init() {
    unsafe {
        SHELL = Some(Shell::new());
    }
}

pub fn execute(line: &[u8]) {
    unsafe {
        if let Some(shell) = (*core::ptr::addr_of_mut!(SHELL)).as_mut() {
            shell.execute(line);
        }
    }
}