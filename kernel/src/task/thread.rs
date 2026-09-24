use crate::{
    arch::x86_64::{
        context::{CpuContext, InterruptFrame, UserInterruptFrame},
        gdt::{GDT_USER_CODE, GDT_USER_DATA},
        paging::PageTable,
    },
    task::{process::ProcessID, thread::ThreadState::Terminated},
};
use alloc::boxed::Box;

pub type ThreadID = u64;

const KERNEL_STACK_SIZE: usize = 16 * 1024;

const USER_RFLAGS: u64 = 0x202;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThreadKind {
    Kernel,
    User,
}

#[repr(align(16))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Ready,
    Running,
    Waiting,
    Sleeping,
    Terminated,
}

pub struct Thread {
    id: ThreadID,
    kind: ThreadKind,
    state: ThreadState,
    started: bool,
    cpu_id: usize,
    context: CpuContext,
    interrupt_rsp: u64,
    stack_top: u64,
    entry: u64,
    stack: Box<KernelStack>,
    wake_at: u64,
    waiting_for: Option<ThreadID>,
    process_id: ProcessID,
    user_rip: u64,
    user_rsp: u64,
    waiting_for_process: Option<ProcessID>,
}

impl Thread {
    pub fn new(
        id: ThreadID,
        cpu_id: usize,
        process_id: ProcessID,
        entry: extern "C" fn() -> !,
    ) -> Self {
        let stack = Box::new(KernelStack {
            data: [0; KERNEL_STACK_SIZE],
        });

        let mut thread = Self {
            id,
            kind: ThreadKind::Kernel,
            state: ThreadState::Ready,
            started: false,
            cpu_id,
            context: CpuContext::empty(),
            interrupt_rsp: 0,
            stack_top: 0,
            entry: entry as u64,
            wake_at: 0,
            stack,
            waiting_for: None,
            process_id,
            user_rip: 0,
            user_rsp: 0,
            waiting_for_process: None,
        };

        thread.setup_stack(entry);

        thread
    }

    pub fn new_user(
        id: ThreadID,
        cpu_id: usize,
        process_id: ProcessID,
        user_rip: u64,
        user_rsp: u64,
    ) -> Self {
        let stack = Box::new(KernelStack {
            data: [0; KERNEL_STACK_SIZE],
        });

        let mut thread = Self {
            id,
            kind: ThreadKind::User,
            state: ThreadState::Ready,
            started: false,
            cpu_id,
            context: CpuContext::empty(),
            interrupt_rsp: 0,
            stack_top: 0,
            entry: user_rip,
            wake_at: 0,
            stack,
            waiting_for: None,
            process_id,
            user_rip,
            user_rsp,
            waiting_for_process: None,
        };

        thread.setup_user_stack(user_rip, user_rsp);

        thread
    }

    pub fn process_id(&self) -> ProcessID {
        self.process_id
    }

    fn setup_stack(&mut self, entry: extern "C" fn() -> !) {
        let stack_top = (self.stack.data.as_ptr() as usize + KERNEL_STACK_SIZE) & !0xF;

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

    pub fn set_waiting_for_process(&mut self, pid: Option<ProcessID>) {
        self.waiting_for_process = pid;
    }

    pub fn waiting_for_process(&self) -> Option<ProcessID> {
        self.waiting_for_process
    }

    fn setup_user_stack(&mut self, user_rip: u64, user_rsp: u64) {
        let stack_top = (self.stack.data.as_ptr() as usize + KERNEL_STACK_SIZE) & !0xF;

        self.stack_top = stack_top as u64;

        let frame_size = core::mem::size_of::<UserInterruptFrame>();
        let frame_addr = stack_top - frame_size;

        unsafe {
            let frame = frame_addr as *mut UserInterruptFrame;

            (*frame).r15 = 0;
            (*frame).r14 = 0;
            (*frame).r13 = 0;
            (*frame).r12 = 0;
            (*frame).r11 = 0;
            (*frame).r10 = 0;
            (*frame).r9 = 0;
            (*frame).r8 = 0;
            (*frame).rbp = 0;
            (*frame).rdi = 0;
            (*frame).rsi = 0;
            (*frame).rdx = 0;
            (*frame).rcx = 0;
            (*frame).rbx = 0;
            (*frame).rax = 0;

            (*frame).rip = user_rip;
            (*frame).cs = GDT_USER_CODE as u64;
            (*frame).rflags = USER_RFLAGS;
            (*frame).rsp = user_rsp;
            (*frame).ss = GDT_USER_DATA as u64;
        }

        self.interrupt_rsp = frame_addr as u64;
    }

    pub fn replace_user_image(&mut self, user_rip: u64, user_rsp: u64) {
        self.kind = ThreadKind::User;
        self.entry = user_rip;
        self.user_rip = user_rip;
        self.user_rsp = user_rsp;
        self.setup_user_stack(user_rip, user_rsp);
    }

    pub fn kind(&self) -> ThreadKind {
        self.kind
    }

    pub fn is_user(&self) -> bool {
        self.kind == ThreadKind::User
    }

    pub fn is_kernel(&self) -> bool {
        self.kind == ThreadKind::Kernel
    }

    pub fn user_rip(&self) -> u64 {
        self.user_rip
    }

    pub fn user_rsp(&self) -> u64 {
        self.user_rsp
    }

    pub fn user_frame_ptr(&self) -> *const UserInterruptFrame {
        self.interrupt_rsp as *const UserInterruptFrame
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

    pub fn wake_at(&self) -> u64 {
        self.wake_at
    }

    pub fn set_wake_at(&mut self, ticks: u64) {
        self.wake_at = ticks;
    }

    pub fn sleep_until(&mut self, wake_at: u64) {
        self.wake_at = wake_at;
        self.state = ThreadState::Sleeping;
    }

    pub fn sleep(&mut self, ticks: u64) {
        let now = crate::arch::x86_64::lapic::ticks();

        self.wake_at = now + ticks;
        self.state = ThreadState::Sleeping;
    }

    pub fn exit(&mut self) {
        self.state = ThreadState::Terminated;
    }

    pub fn is_terminated(&self) -> bool {
        self.state == ThreadState::Terminated
    }

    pub fn waiting_for(&self) -> Option<ThreadID> {
        self.waiting_for
    }

    pub fn set_waiting_for(&mut self, id: Option<ThreadID>) {
        self.waiting_for = id;
    }

    pub fn cpu_id(&self) -> usize {
        self.cpu_id
    }

    pub fn set_cpu_id(&mut self, cpu_id: usize) {
        self.cpu_id = cpu_id;
    }
}
