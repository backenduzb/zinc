use alloc::vec::Vec;

use crate::task::{
    process,
    thread::{self, Thread},
};

use super::thread::ThreadID;

pub type ProcessID = u64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Waiting,
    Terminated,
}

pub struct Process {
    id: ProcessID,
    state: ProcessState,
    threads: Vec<ThreadID>,
}

pub struct ProcessManager {
    processes: Vec<Process>,
    next_pid: ProcessID,
}

impl Process {
    pub fn new(id: ProcessID) -> Self {
        Self {
            id,
            state: ProcessState::Ready,
            threads: Vec::new(),
        }
    }

    pub fn id(&self) -> ProcessID {
        self.id
    }

    pub fn state(&self) -> ProcessState {
        self.state
    }

    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }

    pub fn add_thread(&mut self, thread_id: ThreadID) {
        if !self.threads.contains(&thread_id) {
            self.threads.push(thread_id);
        }
    }

    pub fn remove_thread(&mut self, thread_id: ThreadID) {
        if let Some(index) = self.threads.iter().position(|&id| id == thread_id) {
            self.threads.remove(index);
        }
    }

    pub fn threads(&self) -> &[ThreadID] {
        &self.threads
    }

    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }

    pub fn is_terminated(&self) -> bool {
        self.state == ProcessState::Terminated
    }
}
impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            next_pid: 1,
        }
    }

    pub fn create(&mut self) -> ProcessID {
        let pid = self.next_pid;
        self.next_pid += 1;

        self.processes.push(Process::new(pid));

        pid
    }

    pub fn create_with_pid(&mut self, pid: ProcessID) -> bool {
        if self.get(pid).is_some() {
            return false;
        }

        self.processes.push(Process::new(pid));

        if pid >= self.next_pid {
            self.next_pid = pid + 1;
        }

        true
    }

    pub fn get(&self, pid: ProcessID) -> Option<&Process> {
        self.processes.iter().find(|process| process.id() == pid)
    }

    pub fn get_mut(&mut self, pid: ProcessID) -> Option<&mut Process> {
        self.processes
            .iter_mut()
            .find(|process| process.id() == pid)
    }

    pub fn add_thread(&mut self, pid: ProcessID, thread_id: ThreadID) -> bool {
        if let Some(process) = self.get_mut(pid) {
            process.add_thread(thread_id);
            true
        } else {
            false
        }
    }

    pub fn remove_thread(&mut self, pid: ProcessID, thread_id: ThreadID) -> bool {
        if let Some(process) = self.get_mut(pid) {
            process.remove_thread(thread_id);
            true
        } else {
            false
        }
    }

    pub fn terminate(&mut self, pid: ProcessID) -> bool {
        if let Some(process) = self.get_mut(pid) {
            process.set_state(ProcessState::Terminated);
            true
        } else {
            false
        }
    }

    pub fn count(&self) -> usize {
        self.processes.len()
    }
}
