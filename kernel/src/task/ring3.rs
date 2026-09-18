use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::memory::physical::FrameAllocator;
use crate::memory::user_stack::UserStack;
use crate::task::thread::Thread;

pub const USER_CODE_ADDR: u64 = 0x400000;
pub const USER_DATA_ADDR: u64 = 0x500000;
pub const USER_MARKER: u64 = 0xC0DE_CAFE_1234_5678;

pub const RING3_TID: u64 = 20;
pub const RING3_MONITOR_TID: u64 = 21;

static MARKER_PHYS: AtomicU64 = AtomicU64::new(0);
static RING3_PID: AtomicU64 = AtomicU64::new(0);
static PASS_PRINTED: AtomicBool = AtomicBool::new(false);

fn build_user_blob() -> [u8; 25] {
    let mut blob = [0u8; 25];

    blob[0] = 0x48;
    blob[1] = 0xB8;
    blob[2..10].copy_from_slice(&USER_MARKER.to_le_bytes());

    blob[10] = 0x48;
    blob[11] = 0xBB;
    blob[12..20].copy_from_slice(&USER_DATA_ADDR.to_le_bytes());

    blob[20] = 0x48;
    blob[21] = 0x89;
    blob[22] = 0x03;

    blob[23] = 0xEB;
    blob[24] = 0xFE;

    blob
}

fn validate_user_addrs(
    space: &crate::memory::address_space::AddressSpace,
    rip: u64,
    rsp: u64,
) -> Result<(), &'static str> {
    const USER_MAX: u64 = 0x0000_8000_0000_0000;

    if rip >= USER_MAX {
        return Err("user RIP outside user half");
    }

    if rsp >= USER_MAX {
        return Err("user RSP outside user half");
    }

    if rsp & 0x7 != 0 {
        return Err("user RSP unaligned");
    }

    match space.probe_page(rip) {
        Some((user, _, exec)) if user && exec => {}
        _ => return Err("user RIP not executable-mapped"),
    }

    match space.probe_page(rsp) {
        Some((user, writable, _)) if user && writable => {}
        _ => return Err("user RSP not mapped"),
    }

    Ok(())
}

pub fn prepare_ring3_user_thread() -> bool {
    crate::drivers::serial::println("=== RING 3 USER THREAD TEST ===");

    let mut allocator = FrameAllocator::new();

    let pid = unsafe {
        match (*crate::task::process_manager()).create_user(&mut allocator) {
            Some(pid) => pid,
            None => {
                crate::drivers::serial::println("RING3: create_user FAILED");
                return false;
            }
        }
    };

    RING3_PID.store(pid, Ordering::Relaxed);

    crate::drivers::serial::print("User process created: PID=");
    crate::drivers::serial::print_u64(pid);
    crate::drivers::serial::println("");

    let (code_phys, data_phys, user_rsp, pml4) = unsafe {
        let manager = crate::task::process_manager();
        let process = match (*manager).get_mut(pid) {
            Some(process) => process,
            None => {
                crate::drivers::serial::println("RING3: process lookup FAILED");
                return false;
            }
        };

        let space = match process.address_space_mut() {
            Some(space) => space,
            None => {
                crate::drivers::serial::println("RING3: address space MISSING");
                return false;
            }
        };

        let code_frame = match allocator.alloc_frame() {
            Some(frame) => frame,
            None => {
                crate::drivers::serial::println("RING3: code frame FAILED");
                return false;
            }
        };

        if !space.map_page(&mut allocator, USER_CODE_ADDR, code_frame.address, true, true) {
            crate::drivers::serial::println("RING3: code map FAILED");
            allocator.free_frame(code_frame);
            return false;
        }

        let data_frame = match allocator.alloc_frame() {
            Some(frame) => frame,
            None => {
                crate::drivers::serial::println("RING3: data frame FAILED");
                return false;
            }
        };

        if !space.map_page(&mut allocator, USER_DATA_ADDR, data_frame.address, true, false) {
            crate::drivers::serial::println("RING3: data map FAILED");
            allocator.free_frame(data_frame);
            return false;
        }

        let stack = match UserStack::map(space, &mut allocator) {
            Some(stack) => stack,
            None => {
                crate::drivers::serial::println("RING3: user stack FAILED");
                return false;
            }
        };

        let user_rsp = (stack.top - 8) & !0xF;

        if let Err(reason) = validate_user_addrs(space, USER_CODE_ADDR, user_rsp) {
            crate::drivers::serial::print("RING3: validation FAILED: ");
            crate::drivers::serial::println(reason);
            return false;
        }

        let pml4 = match process.pml4_phys() {
            Some(pml4) => pml4,
            None => {
                crate::drivers::serial::println("RING3: PML4 missing");
                return false;
            }
        };

        (code_frame.address, data_frame.address, user_rsp, pml4)
    };

    let code_va = match crate::boot::limine::physical_to_virtual(code_phys) {
        Some(va) => va,
        None => {
            crate::drivers::serial::println("RING3: code HHDM unavailable");
            return false;
        }
    };

    let data_va = match crate::boot::limine::physical_to_virtual(data_phys) {
        Some(va) => va,
        None => {
            crate::drivers::serial::println("RING3: data HHDM unavailable");
            return false;
        }
    };

    unsafe {
        core::ptr::write_bytes(code_va as *mut u8, 0, 4096);

        let blob = build_user_blob();

        core::ptr::copy_nonoverlapping(blob.as_ptr(), code_va as *mut u8, blob.len());

        core::ptr::write_bytes(data_va as *mut u8, 0, 4096);
    }

    MARKER_PHYS.store(data_phys, Ordering::Relaxed);

    crate::drivers::serial::print("User CR3 loaded: PML4=0x");
    crate::drivers::serial::print_hex_u64(pml4);
    crate::drivers::serial::println(" (at first schedule)");

    let user_thread = Thread::new_user(RING3_TID, 0, pid, USER_CODE_ADDR, user_rsp);

    crate::drivers::serial::print("User thread created: TID=");
    crate::drivers::serial::print_u64(RING3_TID);
    crate::drivers::serial::print(" RIP=0x");
    crate::drivers::serial::print_hex_u64(USER_CODE_ADDR);
    crate::drivers::serial::print(" RSP=0x");
    crate::drivers::serial::print_hex_u64(user_rsp);
    crate::drivers::serial::println("");

    let monitor = Thread::new(RING3_MONITOR_TID, 0, 0, ring3_monitor);

    unsafe {
        let scheduler = crate::task::scheduler();
        let manager = crate::task::process_manager();

        if !(*scheduler).add_thread(user_thread, 0) {
            crate::drivers::serial::println("RING3: add user thread FAILED");
            return false;
        }

        (*manager).add_thread(pid, RING3_TID);

        if !(*scheduler).add_thread(monitor, 0) {
            crate::drivers::serial::println("RING3: add monitor FAILED");
            return false;
        }

        (*manager).add_thread(0, RING3_MONITOR_TID);
    }

    crate::drivers::serial::println("RING3 threads queued on CPU 0 (user + monitor)");

    true
}

fn marker_value() -> Option<u64> {
    let phys = MARKER_PHYS.load(Ordering::Relaxed);

    if phys == 0 {
        return None;
    }

    let va = crate::boot::limine::physical_to_virtual(phys)?;

    Some(unsafe { core::ptr::read_volatile(va as *const u64) })
}

extern "C" fn ring3_monitor() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::serial::println("RING3 monitor started");

    loop {
        match marker_value() {
            Some(value) if value == USER_MARKER => {
                if !PASS_PRINTED.swap(true, Ordering::SeqCst) {
                    unsafe {
                        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
                    }

                    crate::drivers::serial::println("User code executed (marker observed)");
                    crate::drivers::serial::println("Kernel verified user execution marker");
                    crate::drivers::serial::println("Returning/scheduling safely");
                    crate::drivers::serial::println("RING 3 USER THREAD TEST: PASS");

                    unsafe {
                        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
                    }
                }

                crate::task::sleep(50);
            }
            _ => unsafe {
                core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
            },
        }
    }
}
