use core::u64;

use crate::arch::x86_64::context::SyscallFrame;
use crate::boot::limine::physical_to_virtual;
use crate::memory::address_space::AddressSpace;
use crate::task::thread;

use alloc::string::String;
use alloc::vec::Vec;

pub const SYS_WAIT: u64 = 7;
pub const MAX_ARGC: usize = 16;
pub const MAX_ARG_LEN: usize = 256;
pub const SYS_YIELD: u64 = 0;
pub const SYS_EXIT: u64 = 1;
pub const SYS_EXECVE: u64 = 2;
pub const SYS_WRITE: u64 = 3;
pub const SYS_OPEN: u64 = 4;
pub const SYS_READ: u64 = 5;
pub const SYS_CLOSE: u64 = 6;
const MAX_PATH: usize = 256;

const USER_MAX: u64 = 0x0000_8000_0000_0000;
const MAX_WRITE: usize = 1024;

fn is_user_range(ptr: u64, len: u64) -> bool {
    if len == 0 {
        return true;
    }
    if ptr == 0 || ptr >= USER_MAX {
        return false;
    }
    let end = match ptr.checked_add(len) {
        Some(e) => e,
        None => return false,
    };
    if end > USER_MAX {
        return false;
    }
    if end & 0xFFFF_8000_0000_0000 != 0 {
        return false;
    }
    true
}

fn copy_from_user(space: &AddressSpace, user_ptr: u64, len: usize, out: &mut [u8]) -> bool {
    for i in 0..len {
        let va = user_ptr + i as u64;
        let page = va & !0xFFF;
        match space.probe_page(page) {
            Some((user, _, _)) if user => {}
            _ => return false,
        }
        let phys_base = match space.translate_page(page) {
            Some(p) => p,
            None => return false,
        };
        let phys = phys_base + (va & 0xFFF);
        let hva = match physical_to_virtual(phys) {
            Some(v) => v,
            None => return false,
        };
        out[i] = unsafe { core::ptr::read_volatile(hva as *const u8) };
    }
    true
}

fn handle_execve(frame: &mut SyscallFrame) -> (u64, u64) {
    let path_ptr = frame.rdi;
    let argv_ptr = frame.rsi;
    let envp_ptr = frame.rdx;

    crate::log_syscall!("SYSCALL: EXECVE");

    crate::log_exec_raw!("EXECVE: path_ptr=0x");
    crate::log_syscall_raw!("{:016X}", path_ptr);
    crate::log_syscall!("");

    crate::log_exec_raw!("EXECVE: argv_ptr=0x");
    crate::log_syscall_raw!("{:016X}", argv_ptr);
    crate::log_syscall!("");

    crate::log_exec_raw!("EXECVE: envp_ptr=0x");
    crate::log_syscall_raw!("{:016X}", envp_ptr);
    crate::log_syscall!("");

    if path_ptr == 0 {
        crate::log_exec!("EXECVE: invalid path pointer");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let space_ptr: *const AddressSpace = unsafe {
        let sched = crate::task::scheduler();

        let pid = match (*sched).current().map(|t| t.process_id()) {
            Some(pid) => pid,
            None => {
                crate::log_exec!("EXECVE: no current process");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        };

        let pm = crate::task::process_manager();

        match (*pm).get(pid).and_then(|p| p.address_space()) {
            Some(space) => space as *const AddressSpace,
            None => {
                crate::log_exec!("EXECVE: no address space");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    let space = unsafe { &*space_ptr };

    let mut path_buf = [0u8; MAX_PATH];

    let path_len = match copy_user_path(space, path_ptr, &mut path_buf) {
        Some(len) => len,
        None => {
            crate::log_exec!("EXECVE: invalid or unterminated user path");
            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }
    };

    if path_len == 0 {
        crate::log_exec!("EXECVE: empty path");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    crate::log_exec_raw!("EXECVE: path=");
    for &b in &path_buf[..path_len] {
        crate::log_syscall_raw!("{}", (b) as char);
    }
    crate::log_syscall!("");

    if argv_ptr == 0 {
        crate::log_exec!("EXECVE: argv=NULL");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let mut argv_owned: Vec<String> = Vec::new();

    for index in 0..MAX_ARGC {
        let offset = match (index as u64).checked_mul(8) {
            Some(v) => v,
            None => {
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        };

        let entry_ptr = match argv_ptr.checked_add(offset) {
            Some(v) => v,
            None => {
                crate::log_exec!("EXECVE: argv pointer overflow");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        };

        if !is_user_range(entry_ptr, 8) {
            crate::log_exec!("EXECVE: invalid argv pointer");
            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }

        let mut entry_bytes = [0u8; 8];

        if !copy_from_user(space, entry_ptr, 8, &mut entry_bytes) {
            crate::log_vfs!("EXECVE: cannot read argv entry");
            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }

        let user_string_ptr = u64::from_ne_bytes(entry_bytes);

        if user_string_ptr == 0 {
            break;
        }

        let arg = match copy_user_string(space, user_string_ptr, MAX_ARG_LEN) {
            Some(arg) => arg,
            None => {
                crate::log_exec!("EXECVE: invalid argv string");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        };

        argv_owned.push(arg);
    }

    if argv_owned.is_empty() {
        crate::log_exec!("EXECVE: empty argv");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    crate::log_exec_raw!("EXECVE: argc=");
    crate::log_syscall_raw!("{}", argv_owned.len() as u64);
    crate::log_syscall!("");

    let argv_refs: Vec<&str> = argv_owned.iter().map(|arg| arg.as_str()).collect();

    let pid = unsafe {
        let sched = crate::task::scheduler();

        match (*sched).current().map(|t| t.process_id()) {
            Some(pid) => pid,
            None => {
                crate::log_exec!("EXECVE: no current process");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    let elf = match crate::exec::vfs::read_exec_path(&path_buf[..path_len]) {
        Ok(elf) => elf,
        Err(_) => {
            crate::log_vfs!("EXECVE: VFS load failed");
            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }
    };

    let mut frame_allocator = crate::memory::physical::FrameAllocator::new();

    let cpu_id = crate::arch::x86_64::smp::percpu::current_cpu_id();

    let (entry, user_rsp, old_address_space) = {
        let pm = crate::task::process_manager();

        match crate::exec::elf::ElfExecutor::replace_current_process(
            &elf,
            &argv_refs,
            unsafe { &mut *pm },
            &mut frame_allocator,
            pid,
            cpu_id,
        ) {
            Ok(result) => result,

            Err(_) => {
                crate::log_exec!("EXECVE: image replacement failed");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    let old_cr3 = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

    crate::log_exec_raw!("EXECVE: PID=");
    crate::log_syscall_raw!("{}", pid);
    crate::log_syscall_raw!(" path=");
    for &b in &path_buf[..path_len] {
        crate::log_syscall_raw!("{}", (b) as char);
    }
    crate::log_syscall!("");

    let new_pml4 = unsafe {
        let pm = crate::task::process_manager();
        (*pm).get(pid).and_then(|p| p.pml4_phys())
    };
    let new_pml4 = match new_pml4 {
        Some(pml4) => pml4,
        None => {
            crate::log_exec!("EXECVE: new address space missing");
            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }
    };

    crate::log_cr3_raw!("EXECVE: old CR3=0x");
    crate::log_syscall_raw!("{:016X}", old_cr3);
    crate::log_syscall!("");
    crate::log_cr3_raw!("EXECVE: new CR3=0x");
    crate::log_syscall_raw!("{:016X}", new_pml4);
    crate::log_syscall!("");

    unsafe {
        crate::arch::x86_64::paging::load_cr3(new_pml4);
    }

    crate::log_cr3!("EXECVE: new CR3 loaded");

    if let Some(old) = old_address_space {
        old.destroy(&mut frame_allocator);
    }

    let new_rsp = unsafe {
        let sched = crate::task::scheduler();
        match (*sched).current_mut() {
            Some(thread) => {
                thread.replace_user_image(entry, user_rsp);
                thread.interrupt_rsp()
            }
            None => {
                crate::log_vfs!("EXECVE: no current thread");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    crate::log_exec_raw!("EXECVE: image replaced PID=");
    crate::log_syscall_raw!("{}", pid);
    crate::log_syscall_raw!(" entry=0x");
    crate::log_syscall_raw!("{:016X}", entry);
    crate::log_syscall_raw!(" RSP=0x");
    crate::log_syscall_raw!("{:016X}", user_rsp);
    crate::log_syscall!("");

    let trampoline = crate::arch::x86_64::context::start_user_thread_context as usize as u64;
    (new_rsp, trampoline)
}

fn handle_write(frame: &mut SyscallFrame) -> (u64, u64) {
    let fd = frame.rdi;
    let buf = frame.rsi;
    let len = frame.rdx;

    crate::log_syscall_raw!("SYSCALL: WRITE fd=");
    crate::log_syscall_raw!("{}", fd);
    crate::log_syscall_raw!(" len=");
    crate::log_syscall_raw!("{}", len);
    crate::log_syscall!("");

    let fd_ok = unsafe {
        let sched = crate::task::scheduler();

        let pid = match (*sched).current().map(|t| t.process_id()) {
            Some(pid) => pid,
            None => {
                crate::log_syscall!("SYSCALL WRITE: no current thread");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        };

        let pm = crate::task::process_manager();

        match (*pm).get(pid) {
            Some(process) => process.fd_table().is_standard_output(fd as usize),
            None => {
                crate::log_syscall!("SYSCALL WRITE: process not found");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    if !fd_ok {
        crate::log_syscall!("SYSCALL WRITE: bad fd");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    if len == 0 {
        frame.rax = 0;
        return (frame as *mut SyscallFrame as u64, 0);
    }
    if len > MAX_WRITE as u64 {
        crate::log_syscall!("SYSCALL WRITE: len too large");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }
    if !is_user_range(buf, len) {
        crate::log_syscall!("SYSCALL WRITE: bad user range");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let space_ptr: *const AddressSpace = unsafe {
        let sched = crate::task::scheduler();
        let pid = match (*sched).current().map(|t| t.process_id()) {
            Some(p) => p,
            None => {
                crate::log_syscall!("SYSCALL WRITE: no current thread");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        };
        let pm = crate::task::process_manager();
        match (*pm).get(pid).and_then(|p| p.address_space()) {
            Some(s) => s as *const AddressSpace,
            None => {
                crate::log_syscall!("SYSCALL WRITE: no address space");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };
    let space = unsafe { &*space_ptr };

    let ulen = len as usize;
    let mut kbuf = [0u8; MAX_WRITE];
    if !copy_from_user(space, buf, ulen, &mut kbuf[..ulen]) {
        crate::log_syscall!("SYSCALL WRITE: copy_from_user failed");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    crate::log_ring3_raw!("USER OUTPUT: ");
    for &b in &kbuf[..ulen] {
        crate::log_syscall_raw!("{}", (b) as char);
    }

    frame.rax = len;
    (frame as *mut SyscallFrame as u64, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(frame: *mut SyscallFrame) -> (u64, u64) {
    if frame.is_null() {
        return (0, 0);
    }

    let frame = unsafe { &mut *frame };

    if frame.cs & 3 != 3 {
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let cpu = crate::arch::x86_64::smp::percpu::current_cpu_id();

    let tid = unsafe { (*crate::task::scheduler()).current_id() };

    crate::log_syscall_raw!("SYSCALL: CPU=");
    crate::log_syscall_raw!("{}", cpu as u64);
    crate::log_syscall_raw!(" TID=");
    crate::log_syscall_raw!("{}", tid.unwrap_or(0));
    crate::log_syscall_raw!(" nr=");
    crate::log_syscall_raw!("{}", frame.rax);
    crate::log_syscall!("");

    match frame.rax {
        SYS_YIELD => {
            crate::log_syscall!("SYSCALL: YIELD");

            frame.rax = 0;

            unsafe { (*crate::task::scheduler()).syscall_yield(frame as *mut SyscallFrame) }
        }

        SYS_EXIT => {
            crate::log_syscall_raw!("SYSCALL: EXIT code=");
            crate::log_syscall_raw!("{}", frame.rdi);
            crate::log_syscall!("");

            unsafe { (*crate::task::scheduler()).syscall_exit(frame as *mut SyscallFrame) }
        }

        SYS_WRITE => handle_write(frame),

        SYS_OPEN => handle_open(frame),

        SYS_READ => handle_read(frame),

        SYS_CLOSE => handle_close(frame),

        SYS_WAIT => handle_wait(frame),
        
        SYS_EXECVE => handle_execve(frame),


        _ => {
            frame.rax = u64::MAX;

            (frame as *mut SyscallFrame as u64, 0)
        }
    }
}

fn copy_user_path(space: &AddressSpace, user_ptr: u64, out: &mut [u8; MAX_PATH]) -> Option<usize> {
    for i in 0..MAX_PATH {
        if !is_user_range(user_ptr + i as u64, 1) {
            return None;
        }

        let mut byte = [0u8; 1];

        if !copy_from_user(space, user_ptr + i as u64, 1, &mut byte) {
            return None;
        }

        out[i] = byte[0];

        if byte[0] == 0 {
            return Some(i);
        }
    }

    None
}

fn handle_open(frame: &mut SyscallFrame) -> (u64, u64) {
    let path_ptr = frame.rdi;

    crate::log_syscall!("SYSCALL: OPEN");

    let pid = unsafe {
        match (*crate::task::scheduler()).current() {
            Some(thread) => thread.process_id(),
            None => {
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    let space_ptr = unsafe {
        let pm = crate::task::process_manager();

        match (*pm).get(pid) {
            Some(process) => match process.address_space() {
                Some(space) => space as *const AddressSpace,
                None => {
                    frame.rax = u64::MAX;
                    return (frame as *mut SyscallFrame as u64, 0);
                }
            },
            None => {
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    let space = unsafe { &*space_ptr };

    let mut path_buffer = [0u8; MAX_PATH];

    let path_len = match copy_user_path(space, path_ptr, &mut path_buffer) {
        Some(len) => len,
        None => {
            crate::log_syscall!("SYSCALL OPEN: invalid user path");

            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }
    };

    if path_len == 0 {
        crate::log_syscall!("SYSCALL OPEN: empty path");

        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    crate::log_vfs_raw!("OPEN path: ");

    for &byte in &path_buffer[..path_len] {
        crate::log_syscall_raw!("{}", (byte) as char);
    }

    crate::log_syscall!("");

    let fs_ptr = crate::filesystem::context::filesystem();

    if fs_ptr.is_null() {
        crate::log_syscall!("SYSCALL OPEN: filesystem unavailable");

        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let file = unsafe { (*fs_ptr).open(&path_buffer[..path_len]) };

    let file = match file {
        Some(file) => file,
        None => {
            crate::log_syscall!("SYSCALL OPEN: file not found");

            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }
    };

    let fd = unsafe {
        let pm = crate::task::process_manager();

        match (*pm).get_mut(pid) {
            Some(process) => match process.fd_table_mut().alloc(file) {
                Some(fd) => fd,
                None => {
                    crate::log_syscall!("SYSCALL OPEN: fd table full");

                    frame.rax = u64::MAX;
                    return (frame as *mut SyscallFrame as u64, 0);
                }
            },

            None => {
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    crate::log_vfs_raw!("OPEN fd: ");
    crate::log_syscall_raw!("{}", fd as u64);
    crate::log_syscall!("");

    frame.rax = fd as u64;

    (frame as *mut SyscallFrame as u64, 0)
}

fn handle_read(frame: &mut SyscallFrame) -> (u64, u64) {
    let fd = frame.rdi as usize;
    let buf = frame.rsi;
    let len = frame.rdx as usize;

    crate::log_syscall_raw!("SYSCALL: READ fd=");
    crate::log_syscall_raw!("{}", fd as u64);
    crate::log_syscall_raw!(" len=");
    crate::log_syscall_raw!("{}", len as u64);
    crate::log_syscall!("");

    if len == 0 {
        frame.rax = 0;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    if len > MAX_WRITE {
        crate::log_syscall!("SYSCALL READ: len too large");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    if !is_user_range(buf, len as u64) {
        crate::log_syscall!("SYSCALL READ: bad user range");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let pid = unsafe {
        match (*crate::task::scheduler()).current() {
            Some(thread) => thread.process_id(),
            None => {
                crate::log_syscall!("SYSCALL READ: no current thread");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    let space_ptr = unsafe {
        let pm = crate::task::process_manager();

        match (*pm).get(pid).and_then(|p| p.address_space()) {
            Some(space) => space as *const AddressSpace,
            None => {
                crate::log_syscall!("SYSCALL READ: no address space");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    let space = unsafe { &*space_ptr };

    if fd == crate::filesystem::fd::STDIN {
        let is_stdin = unsafe {
            let pm = crate::task::process_manager();

            match (*pm).get(pid) {
                Some(process) => process.fd_table().stdin(),
                None => {
                    crate::log_syscall!("SYSCALL READ: process not found");
                    frame.rax = u64::MAX;
                    return (frame as *mut SyscallFrame as u64, 0);
                }
            }
        };

        if !is_stdin {
            crate::log_syscall!("SYSCALL READ: bad fd");
            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }

        let mut kbuf = [0u8; MAX_WRITE];
        let count = crate::tty::input::read(&mut kbuf[..len]);

        if !copy_to_user(space, buf, count, &kbuf[..count]) {
            crate::log_syscall!("SYSCALL READ: copy_to_user failed");
            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }

        frame.rax = count as u64;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let mut file = unsafe {
        let pm = crate::task::process_manager();

        match (*pm).get_mut(pid) {
            Some(process) => match process.fd_table_mut().get_mut(fd) {
                Some(crate::filesystem::fd::FileDescriptor::File(file)) => core::mem::replace(
                    file,
                    crate::filesystem::vfs::VfsFile {
                        inode_number: file.inode_number,
                        inode: file.inode,
                        offset: file.offset,
                    },
                ),

                _ => {
                    crate::log_syscall!("SYSCALL READ: bad fd");
                    frame.rax = u64::MAX;
                    return (frame as *mut SyscallFrame as u64, 0);
                }
            },

            None => {
                crate::log_syscall!("SYSCALL READ: process not found");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    let mut kbuf = [0u8; MAX_WRITE];

    let bytes_read = {
        let fs_ptr = crate::filesystem::context::filesystem();

        if fs_ptr.is_null() {
            crate::log_syscall!("SYSCALL READ: filesystem unavailable");

            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }

        unsafe { (*fs_ptr).read(&mut file, &mut kbuf[..len]) }
    };

    if !copy_to_user(space, buf, bytes_read, &kbuf[..bytes_read]) {
        crate::log_syscall!("SYSCALL READ: copy_to_user failed");

        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    unsafe {
        let pm = crate::task::process_manager();

        if let Some(process) = (*pm).get_mut(pid) {
            if let Some(crate::filesystem::fd::FileDescriptor::File(slot)) =
                process.fd_table_mut().get_mut(fd)
            {
                *slot = file;
            }
        }
    }

    crate::log_vfs_raw!("READ bytes=");
    crate::log_syscall_raw!("{}", bytes_read as u64);
    crate::log_syscall!("");

    frame.rax = bytes_read as u64;

    (frame as *mut SyscallFrame as u64, 0)
}

fn copy_to_user(space: &AddressSpace, user_ptr: u64, len: usize, input: &[u8]) -> bool {
    if len == 0 {
        return true;
    }

    if len > input.len() {
        return false;
    }

    if !is_user_range(user_ptr, len as u64) {
        return false;
    }

    for i in 0..len {
        let va = user_ptr + i as u64;
        let page = va & !0xFFF;

        match space.probe_page(page) {
            Some((user, writable, _)) if user && writable => {}
            _ => return false,
        }

        let phys_base = match space.translate_page(page) {
            Some(p) => p,
            None => return false,
        };

        let phys = phys_base + (va & 0xFFF);

        let hva = match physical_to_virtual(phys) {
            Some(v) => v,
            None => return false,
        };

        unsafe {
            core::ptr::write_volatile(hva as *mut u8, input[i]);
        }
    }

    true
}

fn handle_close(frame: &mut SyscallFrame) -> (u64, u64) {
    let fd = frame.rdi as usize;

    crate::log_syscall_raw!("SYSCALL: CLOSE fd=");
    crate::log_syscall_raw!("{}", fd as u64);
    crate::log_syscall!("");

    if fd < 3 {
        crate::log_syscall!("SYSCALL CLOSE: cannot close stdio");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let pid = unsafe {
        match (*crate::task::scheduler()).current() {
            Some(thread) => thread.process_id(),
            None => {
                crate::log_syscall!("SYSCALL CLOSE: no current thread");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    let result = unsafe {
        let pm = crate::task::process_manager();

        match (*pm).get_mut(pid) {
            Some(process) => process.fd_table_mut().close(fd),
            None => {
                crate::log_syscall!("SYSCALL CLOSE: process not found");
                None
            }
        }
    };

    match result {
        Some(_) => {
            crate::log_vfs_raw!("CLOSE: fd=");
            crate::log_syscall_raw!("{}", fd as u64);
            crate::log_vfs!(" closed");

            frame.rax = 0;
        }

        None => {
            crate::log_syscall!("SYSCALL CLOSE: bad fd");

            frame.rax = u64::MAX;
        }
    }

    (frame as *mut SyscallFrame as u64, 0)
}

fn copy_user_string(space: &AddressSpace, user_ptr: u64, max_len: usize) -> Option<String> {
    if user_ptr == 0 {
        return None;
    }

    let mut bytes = Vec::new();

    for i in 0..max_len {
        let ptr = user_ptr.checked_add(i as u64)?;

        if !is_user_range(ptr, 1) {
            return None;
        }

        let mut byte = [0u8; 1];

        if !copy_from_user(space, ptr, 1, &mut byte) {
            return None;
        }

        if byte[0] == 0 {
            return String::from_utf8(bytes).ok();
        }

        bytes.push(byte[0]);
    }

    None
}

fn handle_wait(frame: &mut SyscallFrame) -> (u64, u64) {
    crate::log_syscall!("SYSCALL: WAIT");

    let parent_pid = unsafe {
        match (*crate::task::scheduler()).current() {
            Some(thread) => thread.process_id(),
            None => {
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };

    crate::log_syscall_raw!("SYS_WAIT: parent PID=");
    crate::log_syscall_raw!("{}", parent_pid);
    crate::log_syscall!("");
    let terminated_child = unsafe {
        let pm = crate::task::process_manager();

        (*pm).find_terminated_child(parent_pid)
    };

    if let Some(child_pid) = terminated_child {
        crate::log_syscall_raw!("SYS_WAIT: terminated child=");
        crate::log_syscall_raw!("{}", child_pid);
        crate::log_syscall!("");
        crate::log_syscall_raw!("WAIT: terminated child PID=");
        crate::log_syscall_raw!("{}", child_pid);
        crate::log_syscall!("");

        let mut frame_allocator =
            crate::memory::physical::FrameAllocator::new();

        let reaped = unsafe {
            let pm = crate::task::process_manager();

            (*pm).reap(child_pid, &mut frame_allocator)
        };

        if reaped {
            crate::log_exec_raw!("WAIT: reaped PID=");
            crate::log_syscall_raw!("{}", child_pid);
            crate::log_syscall!("");

            frame.rax = child_pid;

            return (frame as *mut SyscallFrame as u64, 0);
        }

        frame.rax = u64::MAX;

        return (frame as *mut SyscallFrame as u64, 0);
    }

    let live_child = unsafe {
        let pm = crate::task::process_manager();

        (*pm).find_live_child(parent_pid)
    };

    let child_pid = match live_child {
        Some(pid) => pid,
        None => {
            crate::log_syscall!("SYS_WAIT: no child");
            if parent_pid == 1 {
                crate::log_syscall_raw!("WAIT: blocking PID=");
                crate::log_syscall_raw!("{}", parent_pid);
                crate::log_syscall!("");
                unsafe {
                    let scheduler = crate::task::scheduler();
                    (*scheduler).wait_for_any_child();
                }
                crate::log_sched!("WAIT: scheduler switch");
                unsafe {
                    let scheduler = crate::task::scheduler();
                    return (*scheduler).syscall_yield(frame as *mut SyscallFrame);
                }
            }
            crate::log_syscall!("WAIT: no child");
            frame.rax = u64::MAX;
            return (frame as *mut SyscallFrame as u64, 0);
        }
    };

    crate::log_syscall_raw!("SYS_WAIT: live child=");
    crate::log_syscall_raw!("{}", child_pid);
    crate::log_syscall!("");
    crate::log_syscall_raw!("WAIT: blocking on child PID=");
    crate::log_syscall_raw!("{}", child_pid);
    crate::log_syscall!("");
    {
        let tid = unsafe { (*crate::task::scheduler()).current_id().unwrap_or(0) };
        crate::log_syscall_raw!("WAIT: blocking parent PID=");
        crate::log_syscall_raw!("{}", parent_pid);
        crate::log_syscall_raw!(" child PID=");
        crate::log_syscall_raw!("{}", child_pid);
        crate::log_syscall_raw!(" TID=");
        crate::log_syscall_raw!("{}", tid);
        crate::log_syscall!("");
    }

    unsafe {
        let scheduler = crate::task::scheduler();

        (*scheduler).wait_for_child(child_pid);
    }

    crate::log_sched!("WAIT: scheduler switch");

    unsafe {
        let scheduler = crate::task::scheduler();

        (*scheduler).syscall_yield(
            frame as *mut SyscallFrame,
        )
    }
}