use crate::arch::x86_64::context::SyscallFrame;
use crate::boot::limine::physical_to_virtual;
use crate::memory::address_space::AddressSpace;

pub const SYS_YIELD: u64 = 0;
pub const SYS_EXIT: u64 = 1;
pub const SYS_EXECVE: u64 = 2;
pub const SYS_WRITE: u64 = 3;

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

fn handle_write(frame: &mut SyscallFrame) -> (u64, u64) {
    let fd = frame.rdi;
    let buf = frame.rsi;
    let len = frame.rdx;

    crate::drivers::serial::print("SYSCALL: WRITE fd=");
    crate::drivers::serial::print_u64(fd);
    crate::drivers::serial::print(" len=");
    crate::drivers::serial::print_u64(len);
    crate::drivers::serial::println("");

    if fd != 1 && fd != 2 {
        crate::drivers::serial::println("SYSCALL WRITE: bad fd");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }
    if len == 0 {
        frame.rax = 0;
        return (frame as *mut SyscallFrame as u64, 0);
    }
    if len > MAX_WRITE as u64 {
        crate::drivers::serial::println("SYSCALL WRITE: len too large");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }
    if !is_user_range(buf, len) {
        crate::drivers::serial::println("SYSCALL WRITE: bad user range");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let space_ptr: *const AddressSpace = unsafe {
        let sched = crate::task::scheduler();
        let pid = match (*sched).current().map(|t| t.process_id()) {
            Some(p) => p,
            None => {
                crate::drivers::serial::println("SYSCALL WRITE: no current thread");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        };
        let pm = crate::task::process_manager();
        match (*pm).get(pid).and_then(|p| p.address_space()) {
            Some(s) => s as *const AddressSpace,
            None => {
                crate::drivers::serial::println("SYSCALL WRITE: no address space");
                frame.rax = u64::MAX;
                return (frame as *mut SyscallFrame as u64, 0);
            }
        }
    };
    let space = unsafe { &*space_ptr };

    let ulen = len as usize;
    let mut kbuf = [0u8; MAX_WRITE];
    if !copy_from_user(space, buf, ulen, &mut kbuf[..ulen]) {
        crate::drivers::serial::println("SYSCALL WRITE: copy_from_user failed");
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    crate::drivers::serial::print("USER OUTPUT: ");
    for &b in &kbuf[..ulen] {
        crate::drivers::serial::putc(b);
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

    crate::drivers::serial::print("SYSCALL: CPU=");
    crate::drivers::serial::print_u64(cpu as u64);
    crate::drivers::serial::print(" TID=");
    crate::drivers::serial::print_u64(tid.unwrap_or(0));
    crate::drivers::serial::print(" nr=");
    crate::drivers::serial::print_u64(frame.rax);
    crate::drivers::serial::println("");

    match frame.rax {
        SYS_YIELD => {
            crate::drivers::serial::println("SYSCALL: YIELD");

            frame.rax = 0;

            unsafe { (*crate::task::scheduler()).syscall_yield(frame as *mut SyscallFrame) }
        }

        SYS_EXIT => {
            crate::drivers::serial::print("SYSCALL: EXIT code=");
            crate::drivers::serial::print_u64(frame.rdi);
            crate::drivers::serial::println("");

            unsafe { (*crate::task::scheduler()).syscall_exit(frame as *mut SyscallFrame) }
        }

        SYS_WRITE => handle_write(frame),

        SYS_EXECVE => {
            crate::drivers::serial::println("SYSCALL: EXECVE");

            let path_ptr = frame.rdi;
            let argv_ptr = frame.rsi;
            let envp_ptr = frame.rdx;

            crate::drivers::serial::print("EXECVE: path_ptr=0x");
            crate::drivers::serial::print_hex_u64(path_ptr);
            crate::drivers::serial::println("");

            crate::drivers::serial::print("EXECVE: argv_ptr=0x");
            crate::drivers::serial::print_hex_u64(argv_ptr);
            crate::drivers::serial::println("");

            crate::drivers::serial::print("EXECVE: envp_ptr=0x");
            crate::drivers::serial::print_hex_u64(envp_ptr);
            crate::drivers::serial::println("");

            if path_ptr == 0 {
                frame.rax = u64::MAX;

                return (frame as *mut SyscallFrame as u64, 0);
            }

            crate::drivers::serial::println("EXECVE: arguments received");

            frame.rax = 0;

            (frame as *mut SyscallFrame as u64, 0)
        }

        _ => {
            frame.rax = u64::MAX;

            (frame as *mut SyscallFrame as u64, 0)
        }
    }
}
