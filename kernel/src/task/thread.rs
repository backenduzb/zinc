use crate::arch::x86_64::context::CpuContext;
use alloc::boxed::Box;

pub type ThreadID = u64;

const KERNEL_STACK_SIZE: usize = 16 * 1024;

#[repr(align(16))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

pub struct Thread {
    id: ThreadID,
    state: ThreadState,
    context: CpuContext,
    stack: Box<KernelStack>,
}

impl Thread {
    pub fn new(id: ThreadID, entry: extern "C" fn() -> !) -> Self {
        let stack = Box::new(KernelStack {
            data: [0; KERNEL_STACK_SIZE],
        });

        let mut thread = Self {
            id,
            state: ThreadState::Ready,
            context: CpuContext::empty(),
            stack,
        };

        thread.setup_stack(entry);

        thread
    }

    fn setup_stack(&mut self, entry: extern "C" fn() -> !) {
        let stack_top =
            self.stack.data.as_ptr() as usize + KERNEL_STACK_SIZE;

        // Bu avval ishlagan variant.
        let mut rsp = (stack_top & !0xF) - 8;

        unsafe {
            // Return address
            rsp -= 8;
            *(rsp as *mut u64) = entry as u64;

            // Fake callee-saved registers
            rsp -= 8;
            *(rsp as *mut u64) = 0; // r15

            rsp -= 8;
            *(rsp as *mut u64) = 0; // r14

            rsp -= 8;
            *(rsp as *mut u64) = 0; // r13

            rsp -= 8;
            *(rsp as *mut u64) = 0; // r12

            rsp -= 8;
            *(rsp as *mut u64) = 0; // rbx

            rsp -= 8;
            *(rsp as *mut u64) = 0; // rbp
        }

        self.context.rsp = rsp as u64;
    }

    pub fn id(&self) -> ThreadID {
        self.id
    }

    pub fn state(&self) -> ThreadState {
        self.state
    }

    pub fn set_state(&mut self, state: ThreadState) {
        self.state = state;
    }

    pub fn context(&self) -> &CpuContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut CpuContext {
        &mut self.context
    }
}

pub extern "C" fn thread_alpha() -> ! {
    crate::drivers::serial::println("Thread ALPHA started!");

    loop {
        crate::drivers::serial::println("Thread ALPHA");
        crate::task::yield_now();
    }
}

pub extern "C" fn thread_beta() -> ! {
    crate::drivers::serial::println("Thread BETA started!");

    loop {
        crate::drivers::serial::println("Thread BETA");
        crate::task::yield_now();
    }
}

pub extern "C" fn thread_silf() -> ! {
    crate::drivers::serial::println("Thread Silf started!");

    loop {
        crate::drivers::serial::println("Thread Silf");
        crate::task::yield_now();
    }
}