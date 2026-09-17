use super::thread::ThreadID;
use crate::memory::{address_space::AddressSpace, physical::FrameAllocator};
use alloc::vec::Vec;

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
    address_space: Option<AddressSpace>,
    threads: Vec<ThreadID>,
}

impl Process {
    pub fn new(id: ProcessID) -> Self {
        Self {
            id,
            state: ProcessState::Ready,
            address_space: None,
            threads: Vec::new(),
        }
    }

    pub fn new_user(id: ProcessID, address_space: AddressSpace) -> Self {
        Self {
            id,
            state: ProcessState::Ready,
            address_space: Some(address_space),
            threads: Vec::new(),
        }
    }

    pub fn address_space(&self) -> Option<&AddressSpace> {
        self.address_space.as_ref()
    }

    pub fn address_space_mut(&mut self) -> Option<&mut AddressSpace> {
        self.address_space.as_mut()
    }

    pub fn has_user_address_space(&self) -> bool {
        self.address_space.is_some()
    }

    pub fn is_kernel(&self) -> bool {
        self.address_space.is_none()
    }

    pub fn is_user(&self) -> bool {
        self.address_space.is_some()
    }

    pub fn pml4_phys(&self) -> Option<u64> {
        self.address_space.as_ref().map(|space| space.pml4().address)
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

pub struct ProcessManager {
    processes: Vec<Process>,
    next_pid: ProcessID,
}

impl ProcessManager {
    pub const fn new() -> Self {
        Self {
            processes: Vec::new(),
            next_pid: 1,
        }
    }

    pub fn create_user(
        &mut self,
        frame_allocator: &mut FrameAllocator,
    ) -> Option<ProcessID> {
        let address_space = AddressSpace::new_user(frame_allocator)?;

        let pid = self.next_pid;
        self.next_pid += 1;

        self.processes.push(Process::new_user(pid, address_space));

        Some(pid)
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
