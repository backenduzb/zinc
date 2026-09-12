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

    pub fn start(&mut self) -> ! {
        self.state = ThreadState::Running;

        let mut current = CpuContext::empty();

        unsafe {
            crate::arch::x86_64::context::switch_context(&mut current, &self.context);
        }

        loop {
            unsafe {
                core::arch::asm!("hlt");
            }
        }
    }

    fn setup_stack(&mut self, entry: extern "C" fn() -> !) {
        let stack_top = self.stack.data.as_ptr() as usize + KERNEL_STACK_SIZE;

        let mut rsp = stack_top & !0xF;

        unsafe {
            rsp -= 8;
            *(rsp as *mut u64) = entry as u64; 

            rsp -= 8;
            *(rsp as *mut u64) = 0; 

            rsp -= 8;
            *(rsp as *mut u64) = 0;

            rsp -= 8;
            *(rsp as *mut u64) = 0; 

            rsp -= 8;
            *(rsp as *mut u64) = 0; 

            rsp -= 8;
            *(rsp as *mut u64) = 0; 

            rsp -= 8;
            *(rsp as *mut u64) = 0; 
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
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

pub extern "C" fn thread_beta() -> ! {
    crate::drivers::serial::println("Thread BETA started!");

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
