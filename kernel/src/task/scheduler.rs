use crate::task::thread::{ThreadID, ThreadState};

use super::thread::Thread;

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

    pub fn next(&mut self) -> Option<usize> {
        if self.count == 0 {
            return None;
        }

        let previous = self.current;

        for _ in 0..MAX_THREADS {
            self.current = (self.current + 1) % MAX_THREADS;

            if let Some(thread) = &self.threads[self.current] {
                if thread.state() == ThreadState::Ready || thread.state() == ThreadState::Running {
                    return Some(self.current);
                }
            }
        }

        self.current = previous;

        Some(previous)
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
}
