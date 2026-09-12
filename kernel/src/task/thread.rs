use crate::arch::x86_64::context::{CpuContext, InterruptFrame};
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
    started: bool,
    context: CpuContext,
    interrupt_rsp: u64,
    stack_top: u64,
    entry: u64,
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
            started: false,
            context: CpuContext::empty(),
            interrupt_rsp: 0,
            stack_top: 0,
            entry: entry as u64,
            stack,
        };

        thread.setup_stack(entry);

        thread
    }

    fn setup_stack(&mut self, entry: extern "C" fn() -> !) {
        let stack_top =
            (self.stack.data.as_ptr() as usize + KERNEL_STACK_SIZE) & !0xF;

        let stack_top = stack_top - 8;

        self.stack_top = stack_top as u64;

        let mut rsp = stack_top;
    
        unsafe {
            rsp -= 8;
            *(rsp as *mut u64) = 0x10202; // RFLAGS: IF = 1, RF = 1
    
            rsp -= 8;
            *(rsp as *mut u64) = 0x08; // CS
    
            rsp -= 8;
            *(rsp as *mut u64) = entry as u64; // RIP

    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // RAX
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // RBX
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // RCX
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // RDX
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // RSI
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // RDI
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // RBP
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // R8
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // R9
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // R10
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // R11
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // R12
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // R13
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // R14
    
            rsp -= 8;
            *(rsp as *mut u64) = 0; // R15
        }
    
        self.interrupt_rsp = rsp as u64;
    }

    pub fn interrupt_rsp(&self) -> u64 {
        self.interrupt_rsp
    }

    pub fn stack_top(&self) -> u64 {
        self.stack_top
    }

    pub fn entry(&self) -> u64 {
        self.entry
    }

    pub fn is_fresh(&self) -> bool {
        !self.started
    }

    pub fn mark_started(&mut self) {
        self.started = true;
    }

    pub fn set_interrupt_rsp(&mut self, rsp: u64) {
        self.interrupt_rsp = rsp;
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
    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::println("Thread ALPHA started!");

    let mut counter: u64 = 0;

    loop {
        counter = counter.wrapping_add(1);

        if counter % 1_000_000 == 0 {
            crate::drivers::serial::print("Thread ALPHA: ");
            crate::drivers::serial::print_u64(counter);
            crate::drivers::serial::println("");
        }

        core::hint::spin_loop();
    }
}

pub extern "C" fn thread_beta() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::println("Thread BETA started!");

    let mut counter: u64 = 0;

    loop {
        counter = counter.wrapping_add(1);

        if counter % 1_000_000 == 0 {
            crate::drivers::serial::print("Thread BETA: ");
            crate::drivers::serial::print_u64(counter);
            crate::drivers::serial::println("");
        }

        core::hint::spin_loop();
    }
}

pub extern "C" fn thread_silf() -> ! {
    unsafe {
        core::arch::asm!("sti");
    }

    crate::drivers::serial::println("Thread SILF started!");

    let mut counter: u64 = 0;

    loop {
        counter = counter.wrapping_add(1);

        if counter % 1_000_000 == 0 {
            crate::drivers::serial::print("Thread SILF: ");
            crate::drivers::serial::print_u64(counter);
            crate::drivers::serial::println("");
        }

        core::hint::spin_loop();
    }
}