use crate::{
    elf::loader::{ElfError, ElfLoader},
    memory::{physical::FrameAllocator, user_stack::UserStack},
    task::{
        process::{ProcessID, ProcessManager},
        thread::ThreadID,
    },
};

pub struct ExecResult {
    pub process_id: ProcessID,
    pub thread_id: ThreadID,
    pub entry: u64,
    /// Initially stack.top -16, now points to argc (initial RSP)
    pub user_stack_top: u64,
    /// Alias for clarity: initial RSP pointing to argc
    pub user_stack_pointer: u64,
}

#[derive(Debug)]
pub enum ExecError {
    Elf(ElfError),
    ProcessCreationFailed,
    ProcessNotFound,
    UserStackCreationFailed,
    StackSetupFailed,
    ThreadCreationFailed,
    ProcessThreadRegistrationFailed,
}

pub struct ElfExecutor;

impl ElfExecutor {
    /// Legacy exec without arguments (argc=0)
    pub fn exec(
        data: &[u8],
        process_manager: &mut ProcessManager,
        frame_allocator: &mut FrameAllocator,
    ) -> Result<ExecResult, ExecError> {
        Self::exec_with_argv(data, &[], process_manager, frame_allocator)
    }

    /// Exec with argv. Builds user stack argc/argv and creates thread with RSP = argc.
    pub fn exec_with_argv(
        data: &[u8],
        argv: &[&str],
        process_manager: &mut ProcessManager,
        frame_allocator: &mut FrameAllocator,
    ) -> Result<ExecResult, ExecError> {
        crate::drivers::serial::println("EXEC: creating user process");

        let pid = process_manager
            .create_user(frame_allocator)
            .ok_or(ExecError::ProcessCreationFailed)?;

        crate::drivers::serial::print("EXEC: PID=");
        crate::drivers::serial::print_u64(pid);
        crate::drivers::serial::println("");

        let inner: Result<(u64, u64), ExecError> = (|| -> Result<(u64, u64), ExecError> {
            let process = process_manager
                .get_mut(pid)
                .ok_or(ExecError::ProcessNotFound)?;
            let address_space = process
                .address_space_mut()
                .ok_or(ExecError::ProcessNotFound)?;

            crate::drivers::serial::println("EXEC: loading ELF");
            let loaded = ElfLoader::load(data, address_space, frame_allocator)
                .map_err(ExecError::Elf)?;

            crate::drivers::serial::print("EXEC: entry=0x");
            crate::drivers::serial::print_u64(loaded.entry);
            crate::drivers::serial::println("");

            crate::drivers::serial::println("EXEC: creating user stack");
            let stack = UserStack::map(address_space, frame_allocator)
                .ok_or(ExecError::UserStackCreationFailed)?;

            crate::drivers::serial::print("EXEC: argv len=");
            crate::drivers::serial::print_u64(argv.len() as u64);
            crate::drivers::serial::println("");

            let user_rsp = stack
                .setup_argv(address_space, argv)
                .ok_or(ExecError::StackSetupFailed)?;

            crate::drivers::serial::print("EXEC: user RSP(argc)=0x");
            crate::drivers::serial::print_u64(user_rsp);
            crate::drivers::serial::println("");
            crate::drivers::serial::print("EXEC: stack top=0x");
            crate::drivers::serial::print_u64(stack.top);
            crate::drivers::serial::println("");

            Ok((loaded.entry, user_rsp))
        })();

        let (entry, user_rsp) = match inner {
            Ok(v) => v,
            Err(e) => {
                process_manager.remove_process(pid, frame_allocator);
                crate::drivers::serial::println("EXEC: cleaned up failed process");
                return Err(e);
            }
        };

        let cpu_id = crate::arch::x86_64::smp::percpu::current_cpu_id();

        crate::drivers::serial::print("EXEC: creating user thread on CPU=");
        crate::drivers::serial::print_u64(cpu_id as u64);
        crate::drivers::serial::println("");

        let tid = {
            let scheduler_ptr = crate::task::scheduler();

            unsafe {
                (*scheduler_ptr).create_user_thread(cpu_id, process_manager, pid, entry, user_rsp)
            }
        };
        let tid = match tid {
            Some(t) => t,
            None => {
                process_manager.remove_process(pid, frame_allocator);
                crate::drivers::serial::println("EXEC: cleaned up failed process (thread)");
                return Err(ExecError::ThreadCreationFailed);
            }
        };

        crate::drivers::serial::print("EXEC: TID=");
        crate::drivers::serial::print_u64(tid);
        crate::drivers::serial::println("");

        crate::drivers::serial::println("EXEC: process ready");

        Ok(ExecResult {
            process_id: pid,
            thread_id: tid,
            entry,
            user_stack_top: user_rsp,
            user_stack_pointer: user_rsp,
        })
    }
}
