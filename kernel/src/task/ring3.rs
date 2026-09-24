use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::memory::physical::FrameAllocator;
use crate::memory::user_stack::UserStack;
use crate::task::thread::Thread;

pub const USER_CODE_ADDR: u64 = 0x400000;
pub const USER_DATA_ADDR: u64 = 0x500000;
pub const USER_MARKER: u64 = 0xC0DE_CAFE_1234_5678;

pub const RING3_TID: u64 = 20;
pub const RING3_MONITOR_TID: u64 = 21;
pub const WAIT_PARENT_TID: u64 = 40;
pub const WAIT_CHILD_TID: u64 = 41;

static MARKER_PHYS: AtomicU64 = AtomicU64::new(0);
static RING3_PID: AtomicU64 = AtomicU64::new(0);
static PASS_PRINTED: AtomicBool = AtomicBool::new(false);

fn build_user_blob() -> [u8; 82] {
    let mut blob = [0u8; 82];
    let mut off = 0;

    blob[off] = 0x48;
    blob[off + 1] = 0xBB;
    blob[off + 2..off + 10].copy_from_slice(&USER_DATA_ADDR.to_le_bytes());
    off += 10;

    blob[off] = 0x48;
    blob[off + 1] = 0xC7;
    blob[off + 2] = 0x03;
    blob[off + 3..off + 7].copy_from_slice(&1u32.to_le_bytes());
    off += 7;

    for progress in [2u32, 3u32] {
        blob[off] = 0x48;
        blob[off + 1] = 0xC7;
        blob[off + 2] = 0xC0;
        blob[off + 3..off + 7].copy_from_slice(&0u32.to_le_bytes());
        off += 7;

        blob[off] = 0xCD;
        blob[off + 1] = 0x80;
        off += 2;

        blob[off] = 0x48;
        blob[off + 1] = 0xC7;
        blob[off + 2] = 0x03;
        blob[off + 3..off + 7].copy_from_slice(&progress.to_le_bytes());
        off += 7;
    }

    blob[off] = 0x48;
    blob[off + 1] = 0xC7;
    blob[off + 2] = 0xC0;
    blob[off + 3..off + 7].copy_from_slice(&0u32.to_le_bytes());
    off += 7;

    blob[off] = 0xCD;
    blob[off + 1] = 0x80;
    off += 2;

    blob[off] = 0x48;
    blob[off + 1] = 0xB8;
    blob[off + 2..off + 10].copy_from_slice(&USER_MARKER.to_le_bytes());
    off += 10;

    blob[off] = 0x48;
    blob[off + 1] = 0x89;
    blob[off + 2] = 0x03;
    off += 3;

    blob[off] = 0x48;
    blob[off + 1] = 0xC7;
    blob[off + 2] = 0xC0;
    blob[off + 3..off + 7].copy_from_slice(&1u32.to_le_bytes());
    off += 7;

    blob[off] = 0xCD;
    blob[off + 1] = 0x80;
    off += 2;

    blob[off] = 0xEB;
    blob[off + 1] = 0xFE;
    off += 2;

    debug_assert_eq!(off, blob.len());

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
        match (*crate::task::process_manager()).create_user(&mut allocator, None) {
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

fn build_init_blob() -> [u8; 11] {
    let mut b = [0u8; 11];
    let mut o = 0;
    b[o] = 0x48; b[o+1] = 0xC7; b[o+2] = 0xC0; b[o+3..o+7].copy_from_slice(&7u32.to_le_bytes()); o+=7;
    b[o] = 0xCD; b[o+1] = 0x80; o+=2;
    b[o] = 0xEB; b[o+1] = 0xF5; o+=2;
    debug_assert_eq!(o, b.len());
    b
}

fn build_wait_child_blob() -> [u8; 30] {
    let mut b = [0u8; 30];
    let mut o = 0;
    b[o] = 0x48; b[o+1] = 0xC7; b[o+2] = 0xC1; b[o+3..o+7].copy_from_slice(&0x00800000u32.to_le_bytes()); o+=7;
    b[o] = 0x48; b[o+1] = 0xFF; b[o+2] = 0xC9; o+=3;
    b[o] = 0x75; b[o+1] = 0xFB; o+=2;
    b[o] = 0x48; b[o+1] = 0xC7; b[o+2] = 0xC0; b[o+3..o+7].copy_from_slice(&1u32.to_le_bytes()); o+=7;
    b[o] = 0x48; b[o+1] = 0xC7; b[o+2] = 0xC7; b[o+3..o+7].copy_from_slice(&0u32.to_le_bytes()); o+=7;
    b[o] = 0xCD; b[o+1] = 0x80; o+=2;
    b[o] = 0xEB; b[o+1] = 0xFE; o+=2;
    debug_assert_eq!(o, b.len());
    b
}

pub fn prepare_wait_blocking_test() -> bool {
    crate::drivers::serial::println("=== INIT TEST ===");
    let mut allocator = FrameAllocator::new();
    let parent_pid = unsafe {
        let pm = crate::task::process_manager();
        if let Some(p) = (*pm).get(1) {
            if p.has_user_address_space() && !p.is_terminated() {
                (*pm).get_mut(1).unwrap().set_parent(None);
                crate::drivers::serial::println("INIT: reusing existing PID=1");
                1
            } else {
                crate::drivers::serial::println("INIT: PID=1 unusable");
                return false;
            }
        } else {
            match (*pm).create_user_with_pid(1, &mut allocator, None) {
                Some(pid) => pid,
                None => {
                    crate::drivers::serial::println("INIT: parent create FAILED");
                    return false;
                }
            }
        }
    };
    let child_pid = unsafe {
        let pm = crate::task::process_manager();
        if (*pm).get(2).is_none() {
            match (*pm).create_user_with_pid(2, &mut allocator, Some(parent_pid)) {
                Some(pid) => pid,
                None => {
                    crate::drivers::serial::println("INIT: child create FAILED");
                    return false;
                }
            }
        } else {
            crate::drivers::serial::println("INIT: PID=2 occupied");
            return false;
        }
    };
    crate::drivers::serial::print("INIT: PID=");
    crate::drivers::serial::print_u64(parent_pid);
    crate::drivers::serial::println(" started");
    crate::drivers::serial::println("INIT: parent=None");
    crate::drivers::serial::println("INIT: waiting for child");
    crate::drivers::serial::print("INIT: parent PID=");
    crate::drivers::serial::print_u64(parent_pid);
    crate::drivers::serial::print(" child PID=");
    crate::drivers::serial::print_u64(child_pid);
    crate::drivers::serial::println("");
    let (p_code, p_rsp, p_pml4) = unsafe {
        let pm = crate::task::process_manager();
        let proc = match (*pm).get_mut(parent_pid) {
            Some(p) => p,
            None => {
                crate::drivers::serial::println("INIT: parent lookup FAIL");
                return false;
            }
        };
        let space = match proc.address_space_mut() {
            Some(s) => s,
            None => {
                crate::drivers::serial::println("INIT: parent AS missing");
                return false;
            }
        };
        let cf = match allocator.alloc_frame() { Some(f) => f, None => { crate::drivers::serial::println("INIT: parent code frame FAIL"); return false; } };
        if !space.map_page(&mut allocator, USER_CODE_ADDR, cf.address, true, true) {
            crate::drivers::serial::println("INIT: parent code map FAIL");
            return false;
        }
        let stack = match UserStack::map(space, &mut allocator) { Some(s) => s, None => { crate::drivers::serial::println("INIT: parent stack FAIL"); return false; } };
        let rsp = (stack.top - 8) & !0xF;
        if validate_user_addrs(space, USER_CODE_ADDR, rsp).is_err() {
            crate::drivers::serial::println("INIT: parent validate FAIL");
            return false;
        }
        let pml4 = proc.pml4_phys().unwrap_or(0);
        (cf.address, rsp, pml4)
    };
    let (c_code, c_rsp, c_pml4) = unsafe {
        let pm = crate::task::process_manager();
        let proc = match (*pm).get_mut(child_pid) {
            Some(p) => p,
            None => {
                crate::drivers::serial::println("INIT: child lookup FAIL");
                return false;
            }
        };
        let space = match proc.address_space_mut() {
            Some(s) => s,
            None => {
                crate::drivers::serial::println("INIT: child AS missing");
                return false;
            }
        };
        let cf = match allocator.alloc_frame() { Some(f) => f, None => { crate::drivers::serial::println("INIT: child code frame FAIL"); return false; } };
        if !space.map_page(&mut allocator, USER_CODE_ADDR, cf.address, true, true) {
            crate::drivers::serial::println("INIT: child code map FAIL");
            return false;
        }
        let stack = match UserStack::map(space, &mut allocator) { Some(s) => s, None => { crate::drivers::serial::println("INIT: child stack FAIL"); return false; } };
        let rsp = (stack.top - 8) & !0xF;
        if validate_user_addrs(space, USER_CODE_ADDR, rsp).is_err() {
            crate::drivers::serial::println("INIT: child validate FAIL");
            return false;
        }
        let pml4 = proc.pml4_phys().unwrap_or(0);
        (cf.address, rsp, pml4)
    };
    let p_va = match crate::boot::limine::physical_to_virtual(p_code) { Some(v) => v, None => { crate::drivers::serial::println("INIT: parent HHDM FAIL"); return false; } };
    let c_va = match crate::boot::limine::physical_to_virtual(c_code) { Some(v) => v, None => { crate::drivers::serial::println("INIT: child HHDM FAIL"); return false; } };
    unsafe {
        core::ptr::write_bytes(p_va as *mut u8, 0, 4096);
        let b = build_init_blob();
        core::ptr::copy_nonoverlapping(b.as_ptr(), p_va as *mut u8, b.len());
        core::ptr::write_bytes(c_va as *mut u8, 0, 4096);
        let b = build_wait_child_blob();
        core::ptr::copy_nonoverlapping(b.as_ptr(), c_va as *mut u8, b.len());
    }
    crate::drivers::serial::print("INIT: parent CR3=0x");
    crate::drivers::serial::print_hex_u64(p_pml4);
    crate::drivers::serial::print(" child CR3=0x");
    crate::drivers::serial::print_hex_u64(c_pml4);
    crate::drivers::serial::println("");
    let p_thread = Thread::new_user(WAIT_PARENT_TID, 0, parent_pid, USER_CODE_ADDR, p_rsp);
    let c_thread = Thread::new_user(WAIT_CHILD_TID, 1, child_pid, USER_CODE_ADDR, c_rsp);
    unsafe {
        let pm = crate::task::process_manager();
        if !crate::task::SCHEDULERS[0].add_thread(p_thread, 0) {
            crate::drivers::serial::println("INIT: parent add_thread FAIL");
            return false;
        }
        (*pm).add_thread(parent_pid, WAIT_PARENT_TID);
        if !crate::task::SCHEDULERS[1].add_thread(c_thread, 1) {
            crate::drivers::serial::println("INIT: child add_thread FAIL");
            return false;
        }
        (*pm).add_thread(child_pid, WAIT_CHILD_TID);
    }
    crate::drivers::serial::print("INIT: parent TID=");
    crate::drivers::serial::print_u64(WAIT_PARENT_TID);
    crate::drivers::serial::print(" CPU0 child TID=");
    crate::drivers::serial::print_u64(WAIT_CHILD_TID);
    crate::drivers::serial::println(" CPU1");
    crate::drivers::serial::println("INIT: threads queued (SMP cross-CPU)");
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
