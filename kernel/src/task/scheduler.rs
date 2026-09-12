use super::thread::Thread;
use crate::arch::x86_64::context::CpuContext;
use crate::task::thread::{ThreadID, ThreadState};

const MAX_THREADS: usize = 8;

pub struct Scheduler {
    threads: [Option<Thread>; MAX_THREADS],
    current: usize,
    count: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            threads: [const { None }; MAX_THREADS],
            current: 0,
            count: 0,
        }
    }

    pub fn add_thread(&mut self, thread: Thread) -> bool {
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

        for offset in 1..=MAX_THREADS {
            let index = (current + offset) % MAX_THREADS;

            if let Some(thread) = &self.threads[index] {
                if thread.state() == ThreadState::Ready || thread.state() == ThreadState::Running {
                    return Some(index);
                }
            }
        }

        Some(current)
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
        if self.count == 0 {
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        self.current = 0;

        if let Some(thread) = self.threads[0].as_mut() {
            thread.set_state(ThreadState::Running);
        }

        let dummy = CpuContext::empty();

        let new_context = match self.threads[0].as_ref() {
            Some(thread) => thread.context() as *const _,
            None => loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            },
        };

        unsafe {
            crate::arch::x86_64::context::switch_context(
                core::ptr::addr_of!(dummy) as *mut _,
                new_context,
            );
        }

        loop {
            unsafe {
                core::arch::asm!("hlt");
            }
        }
    }

    pub fn switch_to(&mut self, next: usize) {
        if next >= MAX_THREADS {
            return;
        }

        if self.threads[next].is_none() {
            return;
        }

        let current = self.current;

        if current == next {
            return;
        }

        let old_context = match self.threads[current].as_mut() {
            Some(thread) => thread.context_mut() as *mut _,
            None => return,
        };

        let new_context = match self.threads[next].as_ref() {
            Some(thread) => thread.context() as *const _,
            None => return,
        };

        if let Some(thread) = self.threads[current].as_mut() {
            thread.set_state(ThreadState::Ready);
        }

        if let Some(thread) = self.threads[next].as_mut() {
            thread.set_state(ThreadState::Running);
        }

        self.current = next;

        unsafe {
            crate::arch::x86_64::context::switch_context(old_context, new_context);
        }
    }

    pub fn yield_now(&mut self) {
        crate::drivers::serial::println("Scheduler: yield");

        let next = match self.next() {
            Some(index) => index,
            None => {
                crate::drivers::serial::println("Scheduler: no next thread");
                return;
            }
        };

        crate::drivers::serial::print("Scheduler: next = ");
        crate::drivers::serial::print_u64(next as u64);
        crate::drivers::serial::println("");

        self.switch_to(next);
    }

    pub fn tick(&mut self) {
        let next = match self.next() {
            Some(index) => index,
            None => return,
        };

        if next != self.current_index() {
            self.switch_to(next);
        }
    }
}
