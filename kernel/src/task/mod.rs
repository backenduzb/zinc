pub mod process;
pub mod thread;
pub mod scheduler;
pub mod test;

use scheduler::Scheduler;

pub static mut SCHEDULER: Scheduler = Scheduler::new();