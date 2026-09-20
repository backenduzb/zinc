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

    pub fn has_no_threads(&self) -> bool {
        self.threads.is_empty()
    }

    pub fn take_address_space(&mut self) -> Option<AddressSpace> {
        self.address_space.take()
    }

    pub fn is_terminated(&self) -> bool {
        self.state == ProcessState::Terminated
    }
}

pub struct ProcessManager {
    processes: Vec<Process>,
    next_pid: ProcessID,
    next_tid: ThreadID,
}

impl ProcessManager {
    pub const fn new() -> Self {
        Self {
            processes: Vec::new(),
            next_pid: 1,
            next_tid: 1,
        }
    }

    pub fn allocate_thread_id(&mut self) -> ThreadID {
        let id = self.next_tid;
        self.next_tid = self.next_tid.wrapping_add(1);
        if self.next_tid == 0 {
            self.next_tid = 1;
        }
        id
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
            if thread_id >= self.next_tid {
                self.next_tid = thread_id.wrapping_add(1);
                if self.next_tid == 0 {
                    self.next_tid = 1;
                }
            }
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

    /// Remove TID from process and if no threads remain, mark Terminated.
    /// Returns true if process became Terminated.
    pub fn notify_thread_terminated(&mut self, pid: ProcessID, tid: ThreadID) -> bool {
        if let Some(process) = self.get_mut(pid) {
            process.remove_thread(tid);
            if process.threads.is_empty() && !process.is_terminated() {
                // Only auto-terminate user processes with no threads;
                // kernel process (no addr space) with no threads should not auto-terminate unless explicitly.
                // But for simplicity, mark any empty process Terminated if it had an address space or was user.
                process.set_state(ProcessState::Terminated);
                crate::drivers::serial::print("PROCESS terminated PID=");
                crate::drivers::serial::print_u64(pid);
                crate::drivers::serial::println("");
                return true;
            }
            return false;
        }
        false
    }

    pub fn has_live_threads(&self, pid: ProcessID) -> bool {
        if let Some(p) = self.get(pid) {
            !p.threads.is_empty()
        } else {
            false
        }
    }

    /// Reap a terminated process with no live threads. Destroys AddressSpace and frees frames.
    pub fn reap(&mut self, pid: ProcessID, frame_allocator: &mut FrameAllocator) -> bool {
        let idx = match self.processes.iter().position(|p| p.id() == pid) {
            Some(i) => i,
            None => return false,
        };
        // Verify terminated
        if !self.processes[idx].is_terminated() {
            crate::drivers::serial::print("REAP FAILED: PID=");
            crate::drivers::serial::print_u64(pid);
            crate::drivers::serial::println(" not terminated");
            return false;
        }
        if !self.processes[idx].threads.is_empty() {
            crate::drivers::serial::print("REAP FAILED: PID=");
            crate::drivers::serial::print_u64(pid);
            crate::drivers::serial::println(" has live threads");
            return false;
        }
        // Take address space ownership
        let space = self.processes[idx].take_address_space();
        if let Some(s) = space {
            crate::drivers::serial::print("REAP: destroying AddressSpace PID=");
            crate::drivers::serial::print_u64(pid);
            crate::drivers::serial::println("");
            s.destroy(frame_allocator);
            crate::drivers::serial::println("ADDRESS SPACE destroyed");
        }
        self.processes.remove(idx);
        crate::drivers::serial::print("PROCESS reaped PID=");
        crate::drivers::serial::print_u64(pid);
        crate::drivers::serial::println("");
        true
    }

    /// Forced removal for error cleanup: destroy AddressSpace regardless of state and remove process.
    pub fn remove_process(&mut self, pid: ProcessID, frame_allocator: &mut FrameAllocator) -> bool {
        let idx = match self.processes.iter().position(|p| p.id() == pid) {
            Some(i) => i,
            None => return false,
        };
        let space = self.processes[idx].take_address_space();
        if let Some(s) = space {
            s.destroy(frame_allocator);
        }
        self.processes.remove(idx);
        crate::drivers::serial::print("PROCESS removed (cleanup) PID=");
        crate::drivers::serial::print_u64(pid);
        crate::drivers::serial::println("");
        true
    }

    pub fn count(&self) -> usize {
        self.processes.len()
    }
}
