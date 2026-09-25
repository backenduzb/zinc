use crate::{
    elf::loader::{ElfError, ElfLoader}, memory::{address_space::AddressSpace, physical::FrameAllocator, user_stack::UserStack}, task::{
        process::{ProcessID, ProcessManager},
        thread::ThreadID,
    },
};

pub struct ExecResult {
    pub process_id: ProcessID,
    pub thread_id: ThreadID,
    pub entry: u64,
    pub user_stack_top: u64,
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
    pub fn exec(
        data: &[u8],
        process_manager: &mut ProcessManager,
        frame_allocator: &mut FrameAllocator,
    ) -> Result<ExecResult, ExecError> {
        Self::exec_with_argv(data, &[], process_manager, frame_allocator)
    }

    pub fn exec_with_argv(
        data: &[u8],
        argv: &[&str],
        process_manager: &mut ProcessManager,
        frame_allocator: &mut FrameAllocator,
    ) -> Result<ExecResult, ExecError> {
        let cpu_id = crate::arch::x86_64::smp::percpu::current_cpu_id();
        Self::exec_with_argv_on_cpu(data, argv, process_manager, frame_allocator, cpu_id)
    }

    pub fn exec_with_argv_on_cpu(
        data: &[u8],
        argv: &[&str],
        process_manager: &mut ProcessManager,
        frame_allocator: &mut FrameAllocator,
        cpu_id: usize,
    ) -> Result<ExecResult, ExecError> {
        crate::log_exec!("EXEC: creating user process");

        let pid = process_manager
            .create_user(frame_allocator, None)
            .ok_or(ExecError::ProcessCreationFailed)?;

        crate::log_exec_raw!("EXEC: PID=");
        crate::log_exec_raw!("{}", pid);
        crate::log_exec!("");

        let inner: Result<(u64, u64), ExecError> = (|| -> Result<(u64, u64), ExecError> {
            let process = process_manager
                .get_mut(pid)
                .ok_or(ExecError::ProcessNotFound)?;
            let address_space = process
                .address_space_mut()
                .ok_or(ExecError::ProcessNotFound)?;

            crate::log_exec!("EXEC: loading ELF");
            let loaded = ElfLoader::load(data, address_space, frame_allocator)
                .map_err(ExecError::Elf)?;

            crate::log_exec_raw!("EXEC: entry=0x");
            crate::log_exec_raw!("{}", loaded.entry);
            crate::log_exec!("");

            crate::log_exec!("EXEC: creating user stack");
            let stack = UserStack::map(address_space, frame_allocator)
                .ok_or(ExecError::UserStackCreationFailed)?;

            crate::log_exec_raw!("EXEC: argv len=");
            crate::log_exec_raw!("{}", argv.len() as u64);
            crate::log_exec!("");

            let user_rsp = stack
                .setup_argv(address_space, argv)
                .ok_or(ExecError::StackSetupFailed)?;

            crate::log_exec_raw!("EXEC: user RSP(argc)=0x");
            crate::log_exec_raw!("{}", user_rsp);
            crate::log_exec!("");
            crate::log_exec_raw!("EXEC: stack top=0x");
            crate::log_exec_raw!("{}", stack.top);
            crate::log_exec!("");

            Ok((loaded.entry, user_rsp))
        })();

        let (entry, user_rsp) = match inner {
            Ok(v) => v,
            Err(e) => {
                process_manager.remove_process(pid, frame_allocator);
                crate::log_exec!("EXEC: cleaned up failed process");
                return Err(e);
            }
        };

        crate::log_vfs_raw!("EXEC: creating user thread on CPU=");
        crate::log_exec_raw!("{}", cpu_id as u64);
        crate::log_exec!("");

        let tid = if cpu_id < crate::arch::x86_64::smp::percpu::MAX_CPUS {
            unsafe {
                (&mut crate::task::SCHEDULERS[cpu_id])
                    .create_user_thread(cpu_id, process_manager, pid, entry, user_rsp)
            }
        } else {
            None
        };
        let tid = match tid {
            Some(t) => t,
            None => {
                process_manager.remove_process(pid, frame_allocator);
                crate::log_vfs!("EXEC: cleaned up failed process (thread)");
                return Err(ExecError::ThreadCreationFailed);
            }
        };

        crate::log_exec_raw!("EXEC: TID=");
        crate::log_exec_raw!("{}", tid);
        crate::log_exec!("");

        crate::log_vfs!("EXEC: process ready");

        Ok(ExecResult {
            process_id: pid,
            thread_id: tid,
            entry,
            user_stack_top: user_rsp,
            user_stack_pointer: user_rsp,
        })
    }

    pub fn replace_current_process(
        data: &[u8],
        argv: &[&str],
        process_manager: &mut ProcessManager,
        frame_allocator: &mut FrameAllocator,
        pid: ProcessID,
        _cpu_id: usize,
    ) -> Result<(u64, u64, Option<AddressSpace>), ExecError> {
        crate::log_exec!("EXEC: preparing replacement image");
    
        let mut new_address_space =
            AddressSpace::new_user(frame_allocator)
                .ok_or(ExecError::ProcessCreationFailed)?;
    
        let loaded = match ElfLoader::load(
            data,
            &mut new_address_space,
            frame_allocator,
        ) {
            Ok(v) => v,
            Err(e) => {
                new_address_space.destroy(frame_allocator);
                return Err(ExecError::Elf(e));
            }
        };
    
        crate::log_exec_raw!("EXEC: replacement entry=0x");
        crate::log_exec_raw!("{}", loaded.entry);
        crate::log_exec!("");
    
        let stack = match UserStack::map(
            &mut new_address_space,
            frame_allocator,
        ) {
            Some(stack) => stack,
            None => {
                new_address_space.destroy(frame_allocator);
                return Err(ExecError::UserStackCreationFailed);
            }
        };
    
        let user_rsp = match stack.setup_argv(
            &mut new_address_space,
            argv,
        ) {
            Some(rsp) => rsp,
            None => {
                new_address_space.destroy(frame_allocator);
                return Err(ExecError::StackSetupFailed);
            }
        };
    
        crate::log_exec_raw!("EXEC: replacement RSP=0x");
        crate::log_exec_raw!("{}", user_rsp);
        crate::log_exec!("");
    
        let process = match process_manager.get_mut(pid) {
            Some(process) => process,
            None => {
                new_address_space.destroy(frame_allocator);
                return Err(ExecError::ProcessNotFound);
            }
        };
    
        let old_address_space = process.replace_address_space(new_address_space);
      
    
        crate::log_exec_raw!("EXEC: image replaced PID=");
        crate::log_exec_raw!("{}", pid);
        crate::log_exec!("");
    
        Ok((loaded.entry, user_rsp, old_address_space))
    }
}
