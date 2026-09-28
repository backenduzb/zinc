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

        commands::execute(&mut self.state, command);
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