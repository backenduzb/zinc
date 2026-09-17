use super::thread::Thread;
use crate::arch::x86_64::context::{CpuContext, InterruptFrame};
use crate::drivers::serial;
use crate::task::thread::{self, ThreadID, ThreadState};

const MAX_THREADS: usize = 8;
const IDLE_SLOT: usize = MAX_THREADS - 1;

pub struct Scheduler {
    threads: [Option<Thread>; MAX_THREADS],
    current: usize,
    count: usize,
    switches: u64,
    ticks: u64,
}

impl Scheduler {
    pub fn init_idle(&mut self, cpu_id: usize) {
        if self.threads[IDLE_SLOT].is_some() {
            return;
        }

        let idle = Thread::new(0, cpu_id, 0, idle_thread);

        self.threads[IDLE_SLOT] = Some(idle);
        self.threads[IDLE_SLOT]
            .as_mut()
            .unwrap()
            .set_state(ThreadState::Ready);

        self.count += 1;
    }

    pub const fn new() -> Self {
        Self {
            threads: [const { None }; MAX_THREADS],
            current: 0,
            count: 0,
            switches: 0,
            ticks: 0,
        }
    }

    pub fn add_thread(&mut self, thread: Thread, cpu_id: usize) -> bool {
        if thread.cpu_id() != cpu_id {
            return false;
        }

        if self.count >= MAX_THREADS {
            return false;
        }

        for slot in self.threads.iter_mut() {
            if slot.is_none() {
                *slot = Some(thread);
                self.count += 1;
                return true;
            }
        }

        false
    }

    pub fn current(&self) -> Option<&Thread> {
        if self.count == 0 {
            return None;
        }

        self.threads[self.current].as_ref()
    }

    pub fn current_mut(&mut self) -> Option<&mut Thread> {
        if self.count == 0 {
            return None;
        }

        self.threads[self.current].as_mut()
    }

    pub fn current_id(&self) -> Option<ThreadID> {
        self.current().map(|thread| thread.id())
    }

    pub fn next(&self) -> Option<usize> {
        if self.count == 0 {
            return None;
        }

        let current = self.current;
        for offset in 1..MAX_THREADS {
            let index = (current + offset) % MAX_THREADS;

            if index == IDLE_SLOT {
                continue;
            }

            if let Some(thread) = &self.threads[index] {
                if thread.state() == ThreadState::Ready {
                    return Some(index);
                }
            }
        }

        if let Some(thread) = &self.threads[IDLE_SLOT] {
            if thread.state() == ThreadState::Ready || thread.state() == ThreadState::Running {
                return Some(IDLE_SLOT);
            }
        }

        None
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn thread(&self, index: usize) -> Option<&Thread> {
        self.threads.get(index).and_then(|thread| thread.as_ref())
    }

    pub fn thread_mut(&mut self, index: usize) -> Option<&mut Thread> {
        self.threads
            .get_mut(index)
            .and_then(|thread| thread.as_mut())
    }

    pub fn current_index(&self) -> usize {
        self.current
    }

    pub fn start_first(&mut self) -> ! {
        let first = (0..MAX_THREADS).find(|&index| self.threads[index].is_some());

        let first = match first {
            Some(index) => index,
            None => loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            },
        };

        self.current = first;

        if let Some(thread) = self.threads[first].as_mut() {
            thread.set_state(ThreadState::Running);
            thread.mark_started();
        }

        crate::drivers::serial::print("Scheduler: starting thread slot ");
        crate::drivers::serial::print_u64(first as u64);
        crate::drivers::serial::println("");

        let (stack_top, entry) = match self.threads[first].as_ref() {
            Some(thread) => (thread.stack_top(), thread.entry()),
            None => loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            },
        };

        unsafe {
            crate::arch::x86_64::context::start_first_thread(stack_top, entry as *const ());
        }
    }

    pub fn tick(&mut self, frame: *mut InterruptFrame) -> (u64, u64) {
        if self.count == 0 {
            return (frame as u64, 0);
        }

        self.reap_terminated();

        if crate::arch::x86_64::smp::percpu::current_cpu_id() != 0 {
            self.ticks = self.ticks.wrapping_add(1);

            if self.ticks % 100 == 0 {
                crate::drivers::serial::print("AP Scheduler tick: CPU=");
                crate::drivers::serial::print_u64(
                    crate::arch::x86_64::smp::percpu::current_cpu_id() as u64,
                );
                crate::drivers::serial::print(" ticks=");
                crate::drivers::serial::print_u64(self.ticks);
                crate::drivers::serial::println("");
            }

            if let Some(thread) = self.threads[self.current].as_mut() {
                thread.set_interrupt_rsp(frame as u64);
            }
            
            let now = crate::arch::x86_64::lapic::ticks();

            for thread in self.threads.iter_mut() {
                if let Some(thread) = thread.as_mut() {
                    if thread.state() == ThreadState::Sleeping && thread.wake_at() <= now {
                        thread.set_state(ThreadState::Ready);
                    }
                }
            }

            let next = match self.next() {
                Some(index) => index,
                None => {
                    return (frame as u64, 0);
                }
            };

            if next == self.current {
                return (frame as u64, 0);
            }

            if let Some(thread) = self.threads[self.current].as_mut() {
                if thread.state() == ThreadState::Running {
                    thread.set_state(ThreadState::Ready);
                }
            }

            if let Some(thread) = self.threads[next].as_mut() {
                thread.set_state(ThreadState::Running);
            }

            self.current = next;
            self.switches = self.switches.wrapping_add(1);

            let fresh = match self.threads[next].as_mut() {
                Some(thread) if thread.is_fresh() => {
                    thread.mark_started();
                    true
                }
                _ => false,
            };

            if fresh {
                let thread = self.threads[next].as_ref().unwrap();

                let stack = thread.stack_top();
                let entry = thread.entry();

                return (stack, entry);
            } else {
                let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

                return (rsp, 0);
            }
        }

        let now = crate::arch::x86_64::lapic::ticks();
        crate::drivers::serial::print("TIME=");
        crate::drivers::serial::print_u64(now);
        crate::drivers::serial::println("");

        for thread in self.threads.iter_mut() {
            if let Some(thread) = thread.as_mut() {
                if thread.state() == ThreadState::Sleeping {
                    crate::drivers::serial::print("SLEEP CHECK thread=");
                    crate::drivers::serial::print_u64(thread.id());

                    crate::drivers::serial::print(" wake=");
                    crate::drivers::serial::print_u64(thread.wake_at());

                    crate::drivers::serial::print(" now=");
                    crate::drivers::serial::print_u64(now);

                    if thread.wake_at() <= now {
                        thread.set_state(ThreadState::Ready);
                        crate::drivers::serial::println(" -> WAKE");
                    } else {
                        crate::drivers::serial::println(" -> WAIT");
                    }
                }
            }
        }

        crate::drivers::serial::print("SCHED tick CPU=");
        crate::drivers::serial::print_u64(crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
        crate::drivers::serial::print(" current=");
        crate::drivers::serial::print_u64(self.current as u64);
        crate::drivers::serial::print(" next=");

        match self.next() {
            Some(index) => crate::drivers::serial::print_u64(index as u64),
            None => crate::drivers::serial::print_u64(99),
        }

        crate::drivers::serial::println("");

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_interrupt_rsp(frame as u64);
        }

        let next = match self.next() {
            Some(index) => index,
            None => {
                return (frame as u64, 0);
            }
        };

        if next == self.current {
            return (frame as u64, 0);
        }

        if let Some(thread) = self.threads[self.current].as_mut() {
            if thread.state() == ThreadState::Running {
                thread.set_state(ThreadState::Ready);
            }
        }

        if let Some(thread) = self.threads[next].as_mut() {
            thread.set_state(ThreadState::Running);
        }

        self.current = next;
        self.switches = self.switches.wrapping_add(1);

        if self.switches % 200 == 0 {
            crate::drivers::serial::print("Scheduler: ");
            crate::drivers::serial::print_u64(self.switches);
            crate::drivers::serial::print(" switches, now on slot ");
            crate::drivers::serial::print_u64(next as u64);
            crate::drivers::serial::println("");
        }

        let fresh = match self.threads[next].as_mut() {
            Some(thread) if thread.is_fresh() => {
                thread.mark_started();
                true
            }
            _ => false,
        };

        if fresh {
            let thread = self.threads[next].as_ref().unwrap();

            let stack = thread.stack_top();
            let entry = thread.entry();

            crate::drivers::serial::print("FRESH SWITCH: slot=");
            crate::drivers::serial::print_u64(next as u64);

            crate::drivers::serial::print(" thread=");
            crate::drivers::serial::print_u64(thread.id());

            crate::drivers::serial::print(" entry=");
            crate::drivers::serial::print_u64(entry);

            crate::drivers::serial::print(" stack=");
            crate::drivers::serial::print_u64(stack);

            crate::drivers::serial::println("");

            (stack, entry)
        } else {
            let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

            (rsp, 0)
        }
    }

    pub fn sleep_current(&mut self, ticks: u64) {
        let now = crate::arch::x86_64::lapic::ticks();

        crate::drivers::serial::print("TIME=");
        crate::drivers::serial::print_u64(now);
        crate::drivers::serial::println("");

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_wake_at(now + ticks);
            thread.set_state(ThreadState::Sleeping);

            crate::drivers::serial::print("SLEEP thread=");
            crate::drivers::serial::print_u64(thread.id());
            crate::drivers::serial::print(" wake_at=");
            crate::drivers::serial::print_u64(now + ticks);
            crate::drivers::serial::println("");
        }
    }

    pub fn exit_current(&mut self) {
        let exiting_id = match self.threads[self.current].as_ref() {
            Some(thread) => thread.id(),
            None => return,
        };

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_state(ThreadState::Terminated);

            serial::print("EXIT thread=");
            serial::print_u64(exiting_id);
            serial::println("");
        }

        for index in 0..MAX_THREADS {
            if index == self.current {
                continue;
            }

            if let Some(thread) = self.threads[index].as_mut() {
                if thread.state() == ThreadState::Waiting
                    && thread.waiting_for() == Some(exiting_id)
                {
                    thread.set_waiting_for(None);
                    thread.set_state(ThreadState::Ready);

                    serial::print("JOIN: waking thread=");
                    serial::print_u64(thread.id());
                    serial::println("");
                }
            }
        }
    }

    pub fn kill(&mut self, id: ThreadID) -> bool {
        for index in 0..MAX_THREADS {
            if let Some(thread) = self.threads[index].as_mut() {
                if thread.id() == id {
                    thread.set_state(ThreadState::Terminated);

                    crate::drivers::serial::print("KILL thread=");
                    crate::drivers::serial::print_u64(id);
                    crate::drivers::serial::println("");

                    return true;
                }
            }
        }

        false
    }

    pub fn reap_terminated(&mut self) {
        for index in 0..MAX_THREADS {
            if index == IDLE_SLOT {
                continue;
            }
            
            if index == self.current {
                continue;
            }

            let should_reap = match &self.threads[index] {
                Some(thread) => thread.state() == ThreadState::Terminated,
                None => false,
            };

            if should_reap {
                let id = self.threads[index]
                    .as_ref()
                    .map(|thread| thread.id())
                    .unwrap_or(0);

                self.threads[index] = None;
                self.count -= 1;

                crate::drivers::serial::print("Scheduler: reaped thread=");
                crate::drivers::serial::print_u64(id);
                crate::drivers::serial::print(" slot=");
                crate::drivers::serial::print_u64(index as u64);
                crate::drivers::serial::println("");
            }
        }
    }

    pub fn join(&mut self, id: ThreadID) {
        let current_id = match self.current_id() {
            Some(id) => id,
            None => return,
        };

        if current_id == id {
            return;
        }

        let target_exists = self
            .threads
            .iter()
            .flatten()
            .any(|thread| thread.id() == id);

        if !target_exists {
            return;
        }

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_waiting_for(Some(id));
            thread.set_state(ThreadState::Waiting);

            serial::println("JOIN: current thread waiting");
        }

        loop {
            unsafe {
                core::arch::asm!("hlt");
            }

            let state = self.threads[self.current]
                .as_ref()
                .map(|thread| thread.state());

            if state != Some(ThreadState::Waiting) {
                break;
            }
        }
    }
}

extern "C" fn idle_thread() -> ! {
    loop {
        unsafe {
            core::arch::asm!("sti");
            core::arch::asm!("hlt");
        }
    }
}
