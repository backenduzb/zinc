use crate::arch::x86_64::context::SyscallFrame;

pub const SYS_YIELD: u64 = 0;
pub const SYS_EXIT: u64 = 1;

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(
    frame: *mut SyscallFrame,
) -> (u64, u64) {
    if frame.is_null() {
        return (0, 0);
    }

    let frame = unsafe { &mut *frame };

    if frame.cs & 3 != 3 {
        frame.rax = u64::MAX;
        return (frame as *mut SyscallFrame as u64, 0);
    }

    let cpu = crate::arch::x86_64::smp::percpu::current_cpu_id();

    let tid = unsafe {
        (*crate::task::scheduler()).current_id()
    };

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

            unsafe {
                (*crate::task::scheduler()).syscall_yield(
                    frame as *mut SyscallFrame
                )
            }
        }

        SYS_EXIT => {
            crate::drivers::serial::println("SYSCALL: EXIT");

            unsafe {
                (*crate::task::scheduler()).syscall_exit(
                    frame as *mut SyscallFrame
                )
            }
        }

        _ => {
            frame.rax = u64::MAX;

            (
                frame as *mut SyscallFrame as u64,
                0,
            )
        }
    }
}