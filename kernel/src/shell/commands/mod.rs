pub mod filesystem;
pub mod process;
pub mod shell;

use super::ShellState;
use super::parser::ParseCommand;
use crate::drivers::serial;

pub fn execute(state: &mut ShellState, command: ParseCommand) {
    match command.command() {
        Some(b"help") => shell::help(),
        Some(b"clear") => shell::clear(),
        Some(b"pwd") => shell::pwd(state),
        Some(b"cd") => {
            let path = if command.argc > 1 {
                command.argv[1]
            } else {
                b"/"
            };

            shell::cd(state, path);
        }
        Some(b"echo") => shell::echo(&command),

        Some(b"ls") => filesystem::ls(&command, state),
        Some(b"cat") => filesystem::cat(&command, state),
        Some(b"touch") => filesystem::touch(&command, state),
        Some(b"write") => filesystem::write(&command, state),

        Some(b"exec") => process::exec(&command, state),

        _ => serial::println("command not found"),
    }
}
