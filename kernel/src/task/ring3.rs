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
    crate::log_vfs!("=== RING 3 USER THREAD TEST ===");

    let mut allocator = FrameAllocator::new();

    let pid = unsafe {
        match (*crate::task::process_manager()).create_user(&mut allocator, None) {
            Some(pid) => pid,
            None => {
                crate::log_ring3!("RING3: create_user FAILED");
                return false;
            }
        }
    };

    RING3_PID.store(pid, Ordering::Relaxed);

    crate::log_ring3_raw!("User process created: PID=");
    crate::log_boot_raw!("{}", pid);
    crate::log_boot!("");

    let (code_phys, data_phys, user_rsp, pml4) = unsafe {
        let manager = crate::task::process_manager();
        let process = match (*manager).get_mut(pid) {
            Some(process) => process,
            None => {
                crate::log_ring3!("RING3: process lookup FAILED");
                return false;
            }
        };

        let space = match process.address_space_mut() {
            Some(space) => space,
            None => {
                crate::log_ring3!("RING3: address space MISSING");
                return false;
            }
        };

        let code_frame = match allocator.alloc_frame() {
            Some(frame) => frame,
            None => {
                crate::log_ring3!("RING3: code frame FAILED");
                return false;
            }
        };

        if !space.map_page(&mut allocator, USER_CODE_ADDR, code_frame.address, true, true) {
            crate::log_ring3!("RING3: code map FAILED");
            allocator.free_frame(code_frame);
            return false;
        }

        let data_frame = match allocator.alloc_frame() {
            Some(frame) => frame,
            None => {
                crate::log_ring3!("RING3: data frame FAILED");
                return false;
            }
        };

        if !space.map_page(&mut allocator, USER_DATA_ADDR, data_frame.address, true, false) {
            crate::log_ring3!("RING3: data map FAILED");
            allocator.free_frame(data_frame);
            return false;
        }

        let stack = match UserStack::map(space, &mut allocator) {
            Some(stack) => stack,
            None => {
                crate::log_ring3!("RING3: user stack FAILED");
                return false;
            }
        };

        let user_rsp = (stack.top - 8) & !0xF;

        if let Err(reason) = validate_user_addrs(space, USER_CODE_ADDR, user_rsp) {
            crate::log_ring3_raw!("RING3: validation FAILED: ");
            crate::log_boot!("{}", reason);
            return false;
        }

        let pml4 = match process.pml4_phys() {
            Some(pml4) => pml4,
            None => {
                crate::log_ring3!("RING3: PML4 missing");
                return false;
            }
        };

        (code_frame.address, data_frame.address, user_rsp, pml4)
    };

    let code_va = match crate::boot::limine::physical_to_virtual(code_phys) {
        Some(va) => va,
        None => {
            crate::log_ring3!("RING3: code HHDM unavailable");
            return false;
        }
    };

    let data_va = match crate::boot::limine::physical_to_virtual(data_phys) {
        Some(va) => va,
        None => {
            crate::log_ring3!("RING3: data HHDM unavailable");
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

    crate::log_cr3_raw!("User CR3 loaded: PML4=0x");
    crate::log_boot_raw!("{:016X}", pml4);
    crate::log_sched!(" (at first schedule)");

    let user_thread = Thread::new_user(RING3_TID, 0, pid, USER_CODE_ADDR, user_rsp);

    crate::log_vfs_raw!("User thread created: TID=");
    crate::log_boot_raw!("{}", RING3_TID);
    crate::log_boot_raw!(" RIP=0x");
    crate::log_boot_raw!("{:016X}", USER_CODE_ADDR);
    crate::log_boot_raw!(" RSP=0x");
    crate::log_boot_raw!("{:016X}", user_rsp);
    crate::log_boot!("");

    let monitor = Thread::new(RING3_MONITOR_TID, 0, 0, ring3_monitor);

    unsafe {
        let scheduler = crate::task::scheduler();
        let manager = crate::task::process_manager();

        if !(*scheduler).add_thread(user_thread, 0) {
            crate::log_vfs!("RING3: add user thread FAILED");
            return false;
        }

        (*manager).add_thread(pid, RING3_TID);

        if !(*scheduler).add_thread(monitor, 0) {
            crate::log_ring3!("RING3: add monitor FAILED");
            return false;
        }

        (*manager).add_thread(0, RING3_MONITOR_TID);
    }

    crate::log_vfs!("RING3 threads queued on CPU 0 (user + monitor)");

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
    crate::log_test!("=== INIT TEST ===");
    let mut allocator = FrameAllocator::new();
    let parent_pid = unsafe {
        let pm = crate::task::process_manager();
        if let Some(p) = (*pm).get(1) {
            if p.has_user_address_space() && !p.is_terminated() {
                (*pm).get_mut(1).unwrap().set_parent(None);
                crate::log_boot!("INIT: reusing existing PID=1");
                1
            } else {
                crate::log_boot!("INIT: PID=1 unusable");
                return false;
            }
        } else {
            match (*pm).create_user_with_pid(1, &mut allocator, None) {
                Some(pid) => pid,
                None => {
                    crate::log_test!("INIT: parent create FAILED");
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
                    crate::log_test!("INIT: child create FAILED");
                    return false;
                }
            }
        } else {
            crate::log_boot!("INIT: PID=2 occupied");
            return false;
        }
    };
    crate::log_boot_raw!("INIT: PID=");
    crate::log_boot_raw!("{}", parent_pid);
    crate::log_boot!(" started");
    crate::log_boot!("INIT: parent=None");
    crate::log_boot!("INIT: waiting for child");
    crate::log_boot_raw!("INIT: parent PID=");
    crate::log_boot_raw!("{}", parent_pid);
    crate::log_boot_raw!(" child PID=");
    crate::log_boot_raw!("{}", child_pid);
    crate::log_boot!("");
    let (p_code, p_rsp, p_pml4) = unsafe {
        let pm = crate::task::process_manager();
        let proc = match (*pm).get_mut(parent_pid) {
            Some(p) => p,
            None => {
                crate::log_test!("INIT: parent lookup FAIL");
                return false;
            }
        };
        let space = match proc.address_space_mut() {
            Some(s) => s,
            None => {
                crate::log_boot!("INIT: parent AS missing");
                return false;
            }
        };
        let cf = match allocator.alloc_frame() { Some(f) => f, None => { crate::log_test!("INIT: parent code frame FAIL"); return false; } };
        if !space.map_page(&mut allocator, USER_CODE_ADDR, cf.address, true, true) {
            crate::log_test!("INIT: parent code map FAIL");
            return false;
        }
        let stack = match UserStack::map(space, &mut allocator) { Some(s) => s, None => { crate::log_test!("INIT: parent stack FAIL"); return false; } };
        let rsp = (stack.top - 8) & !0xF;
        if validate_user_addrs(space, USER_CODE_ADDR, rsp).is_err() {
            crate::log_test!("INIT: parent validate FAIL");
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
                crate::log_test!("INIT: child lookup FAIL");
                return false;
            }
        };
        let space = match proc.address_space_mut() {
            Some(s) => s,
            None => {
                crate::log_boot!("INIT: child AS missing");
                return false;
            }
        };
        let cf = match allocator.alloc_frame() { Some(f) => f, None => { crate::log_test!("INIT: child code frame FAIL"); return false; } };
        if !space.map_page(&mut allocator, USER_CODE_ADDR, cf.address, true, true) {
            crate::log_test!("INIT: child code map FAIL");
            return false;
        }
        let stack = match UserStack::map(space, &mut allocator) { Some(s) => s, None => { crate::log_test!("INIT: child stack FAIL"); return false; } };
        let rsp = (stack.top - 8) & !0xF;
        if validate_user_addrs(space, USER_CODE_ADDR, rsp).is_err() {
            crate::log_test!("INIT: child validate FAIL");
            return false;
        }
        let pml4 = proc.pml4_phys().unwrap_or(0);
        (cf.address, rsp, pml4)
    };
    let p_va = match crate::boot::limine::physical_to_virtual(p_code) { Some(v) => v, None => { crate::log_boot!("INIT: parent HHDM FAIL"); return false; } };
    let c_va = match crate::boot::limine::physical_to_virtual(c_code) { Some(v) => v, None => { crate::log_boot!("INIT: child HHDM FAIL"); return false; } };
    unsafe {
        core::ptr::write_bytes(p_va as *mut u8, 0, 4096);
        let b = build_init_blob();
        core::ptr::copy_nonoverlapping(b.as_ptr(), p_va as *mut u8, b.len());
        core::ptr::write_bytes(c_va as *mut u8, 0, 4096);
        let b = build_wait_child_blob();
        core::ptr::copy_nonoverlapping(b.as_ptr(), c_va as *mut u8, b.len());
    }
    crate::log_cr3_raw!("INIT: parent CR3=0x");
    crate::log_boot_raw!("{:016X}", p_pml4);
    crate::log_cr3_raw!(" child CR3=0x");
    crate::log_boot_raw!("{:016X}", c_pml4);
    crate::log_boot!("");
    let p_thread = Thread::new_user(WAIT_PARENT_TID, 0, parent_pid, USER_CODE_ADDR, p_rsp);
    let c_thread = Thread::new_user(WAIT_CHILD_TID, 1, child_pid, USER_CODE_ADDR, c_rsp);
    unsafe {
        let pm = crate::task::process_manager();
        if !crate::task::SCHEDULERS[0].add_thread(p_thread, 0) {
            crate::log_vfs!("INIT: parent add_thread FAIL");
            return false;
        }
        (*pm).add_thread(parent_pid, WAIT_PARENT_TID);
        if !crate::task::SCHEDULERS[1].add_thread(c_thread, 1) {
            crate::log_vfs!("INIT: child add_thread FAIL");
            return false;
        }
        (*pm).add_thread(child_pid, WAIT_CHILD_TID);
    }
    crate::log_boot_raw!("INIT: parent TID=");
    crate::log_boot_raw!("{}", WAIT_PARENT_TID);
    crate::log_smp_raw!(" CPU0 child TID=");
    crate::log_boot_raw!("{}", WAIT_CHILD_TID);
    crate::log_smp!(" CPU1");
    crate::log_vfs!("INIT: threads queued (SMP cross-CPU)");
    true
}

pub const GFX_CODE_ADDR: u64 = 0x400000;
pub const GFX_DATA_ADDR: u64 = 0x500000;
pub const GFX_RECT_OFF: u64 = 64;
pub const GFX_BLIT_OFF: u64 = 96;
pub const GFX_PIX_OFF: u64 = 128;
pub const GFX_MARK_OFF: u64 = 160;
pub const GFX_MARK_PASS: u64 = 0x4746585F50415353;
pub const GFX_MARK_FAIL: u64 = 0x4746585F4641494C;
pub const GFX_CLEAR_COLOR: u32 = 0x00336699;

pub static GFX_MARKER_PHYS: AtomicU64 = AtomicU64::new(0);
pub static GFX_COMPLETE: AtomicBool = AtomicBool::new(false);
pub static GFX_PASSED: AtomicBool = AtomicBool::new(false);

fn gfx_mov_rax_u32(out: &mut alloc::vec::Vec<u8>, value: u32) {
    out.extend_from_slice(&[0x48, 0xC7, 0xC0]);
    out.extend_from_slice(&value.to_le_bytes());
}

fn gfx_mov_rdi_u32(out: &mut alloc::vec::Vec<u8>, value: u32) {
    out.extend_from_slice(&[0x48, 0xC7, 0xC7]);
    out.extend_from_slice(&value.to_le_bytes());
}

fn gfx_mov_rsi_u32(out: &mut alloc::vec::Vec<u8>, value: u32) {
    out.extend_from_slice(&[0x48, 0xC7, 0xC6]);
    out.extend_from_slice(&value.to_le_bytes());
}

fn gfx_mov_rdx_u32(out: &mut alloc::vec::Vec<u8>, value: u32) {
    out.extend_from_slice(&[0x48, 0xC7, 0xC2]);
    out.extend_from_slice(&value.to_le_bytes());
}

fn gfx_mov_eax_u32(out: &mut alloc::vec::Vec<u8>, value: u32) {
    out.extend_from_slice(&[0xB8]);
    out.extend_from_slice(&value.to_le_bytes());
}

fn gfx_movabs_rax(out: &mut alloc::vec::Vec<u8>, value: u64) {
    out.extend_from_slice(&[0x48, 0xB8]);
    out.extend_from_slice(&value.to_le_bytes());
}

fn gfx_movabs_rbx(out: &mut alloc::vec::Vec<u8>, value: u64) {
    out.extend_from_slice(&[0x48, 0xBB]);
    out.extend_from_slice(&value.to_le_bytes());
}

fn gfx_syscall_checked(out: &mut alloc::vec::Vec<u8>, fixups: &mut alloc::vec::Vec<usize>) {
    out.extend_from_slice(&[0xCD, 0x80, 0x48, 0x83, 0xF8, 0x00, 0x75, 0x00]);
    fixups.push(out.len() - 1);
}

fn gfx_emit_exit(out: &mut alloc::vec::Vec<u8>) {
    gfx_mov_eax_u32(out, 1);
    out.extend_from_slice(&[0x31, 0xFF, 0xCD, 0x80, 0xEB, 0xFE]);
}

fn build_gfx_blob() -> alloc::vec::Vec<u8> {
    let mut blob: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
    let mut fixups: alloc::vec::Vec<usize> = alloc::vec::Vec::new();
    gfx_mov_rax_u32(&mut blob, 8);
    gfx_mov_rdi_u32(&mut blob, GFX_DATA_ADDR as u32);
    gfx_mov_rsi_u32(&mut blob, 40);
    gfx_syscall_checked(&mut blob, &mut fixups);
    gfx_mov_eax_u32(&mut blob, 9);
    gfx_mov_rdi_u32(&mut blob, GFX_CLEAR_COLOR);
    gfx_syscall_checked(&mut blob, &mut fixups);
    gfx_mov_eax_u32(&mut blob, 10);
    gfx_mov_rdi_u32(&mut blob, (GFX_DATA_ADDR + GFX_RECT_OFF) as u32);
    gfx_mov_rsi_u32(&mut blob, 20);
    gfx_syscall_checked(&mut blob, &mut fixups);
    gfx_mov_eax_u32(&mut blob, 11);
    gfx_mov_rdi_u32(&mut blob, (GFX_DATA_ADDR + GFX_BLIT_OFF) as u32);
    gfx_mov_rsi_u32(&mut blob, 16);
    gfx_mov_rdx_u32(&mut blob, (GFX_DATA_ADDR + GFX_PIX_OFF) as u32);
    gfx_syscall_checked(&mut blob, &mut fixups);
    gfx_movabs_rax(&mut blob, GFX_MARK_PASS);
    gfx_movabs_rbx(&mut blob, GFX_DATA_ADDR + GFX_MARK_OFF);
    blob.extend_from_slice(&[0x48, 0x89, 0x03]);
    gfx_emit_exit(&mut blob);
    let fail_pos = blob.len();
    gfx_movabs_rax(&mut blob, GFX_MARK_FAIL);
    gfx_movabs_rbx(&mut blob, GFX_DATA_ADDR + GFX_MARK_OFF);
    blob.extend_from_slice(&[0x48, 0x89, 0x03]);
    gfx_emit_exit(&mut blob);
    for pos in fixups {
        let disp = fail_pos as i64 - (pos as i64 + 1);
        if disp < -128 || disp > 127 {
            return alloc::vec::Vec::new();
        }
        blob[pos] = disp as i8 as u8;
    }
    blob
}

fn gfx_marker_value() -> Option<u64> {
    let phys = GFX_MARKER_PHYS.load(Ordering::Relaxed);
    if phys == 0 {
        return None;
    }
    let va = crate::boot::limine::physical_to_virtual(phys)?;
    Some(unsafe { core::ptr::read_volatile((va + GFX_MARK_OFF) as *const u64) })
}

extern "C" fn gfx_monitor() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }
    crate::log_ring3!("GFX monitor started");
    let mut waited = 0u32;
    loop {
        match gfx_marker_value() {
            Some(value) if value == GFX_MARK_PASS => {
                crate::log_vfs!("GFX RING3 TEST: PASS");
                GFX_PASSED.store(true, Ordering::Release);
                GFX_COMPLETE.store(true, Ordering::Release);
                crate::task::exit();
            }
            Some(value) if value == GFX_MARK_FAIL => {
                crate::log_test!("GFX RING3 TEST: guest reported FAIL");
                GFX_PASSED.store(false, Ordering::Release);
                GFX_COMPLETE.store(true, Ordering::Release);
                crate::task::exit();
            }
            _ => {}
        }
        if waited >= 200 {
            crate::log_test!("GFX RING3 TEST: TIMEOUT");
            GFX_PASSED.store(false, Ordering::Release);
            GFX_COMPLETE.store(true, Ordering::Release);
            crate::task::exit();
        }
        waited += 1;
        crate::task::sleep(5);
    }
}

fn gfx_pick_cpu() -> Option<usize> {
    unsafe {
        for cpu in 1..crate::arch::x86_64::smp::percpu::MAX_CPUS {
            if !crate::arch::x86_64::smp::percpu::is_online(cpu) {
                continue;
            }
            if crate::task::SCHEDULERS[cpu].count() < 6 {
                return Some(cpu);
            }
        }
    }
    None
}

pub fn prepare_gfx_syscall_test() -> bool {
    crate::log_vfs!("=== GFX RING3 TEST ===");
    GFX_COMPLETE.store(false, Ordering::Release);
    GFX_PASSED.store(false, Ordering::Release);
    let blob = build_gfx_blob();
    if blob.is_empty() || blob.len() > 4096 {
        crate::log_test!("GFX TEST: blob build FAILED");
        return false;
    }
    let cpu = match gfx_pick_cpu() {
        Some(cpu) => cpu,
        None => {
            crate::log_test!("GFX TEST: no CPU slot");
            return false;
        }
    };
    let mut allocator = FrameAllocator::new();
    let pid = unsafe {
        match (*crate::task::process_manager()).create_user(&mut allocator, None) {
            Some(pid) => pid,
            None => {
                crate::log_test!("GFX TEST: create_user FAILED");
                return false;
            }
        }
    };
    let (code_phys, data_phys, user_rsp) = unsafe {
        let manager = crate::task::process_manager();
        let process = match (*manager).get_mut(pid) {
            Some(process) => process,
            None => {
                crate::log_test!("GFX TEST: process lookup FAILED");
                return false;
            }
        };
        let space = match process.address_space_mut() {
            Some(space) => space,
            None => {
                crate::log_test!("GFX TEST: address space MISSING");
                return false;
            }
        };
        let code_frame = match allocator.alloc_frame() {
            Some(frame) => frame,
            None => {
                crate::log_test!("GFX TEST: code frame FAILED");
                return false;
            }
        };
        if !space.map_page(
            &mut allocator,
            GFX_CODE_ADDR,
            code_frame.address,
            true,
            true,
        ) {
            crate::log_test!("GFX TEST: code map FAILED");
            allocator.free_frame(code_frame);
            return false;
        }
        let data_frame = match allocator.alloc_frame() {
            Some(frame) => frame,
            None => {
                crate::log_test!("GFX TEST: data frame FAILED");
                return false;
            }
        };
        if !space.map_page(
            &mut allocator,
            GFX_DATA_ADDR,
            data_frame.address,
            true,
            false,
        ) {
            crate::log_test!("GFX TEST: data map FAILED");
            allocator.free_frame(data_frame);
            return false;
        }
        let stack = match UserStack::map(space, &mut allocator) {
            Some(stack) => stack,
            None => {
                crate::log_test!("GFX TEST: user stack FAILED");
                return false;
            }
        };
        let user_rsp = (stack.top - 8) & !0xF;
        if validate_user_addrs(space, GFX_CODE_ADDR, user_rsp).is_err() {
            crate::log_test!("GFX TEST: validation FAILED");
            return false;
        }
        (code_frame.address, data_frame.address, user_rsp)
    };
    let code_va = match crate::boot::limine::physical_to_virtual(code_phys) {
        Some(va) => va,
        None => {
            crate::log_test!("GFX TEST: code HHDM unavailable");
            return false;
        }
    };
    let data_va = match crate::boot::limine::physical_to_virtual(data_phys) {
        Some(va) => va,
        None => {
            crate::log_test!("GFX TEST: data HHDM unavailable");
            return false;
        }
    };
    unsafe {
        core::ptr::write_bytes(code_va as *mut u8, 0, 4096);
        core::ptr::copy_nonoverlapping(blob.as_ptr(), code_va as *mut u8, blob.len());
        core::ptr::write_bytes(data_va as *mut u8, 0, 4096);
        let rect = crate::graphics::abi::GfxRect {
            x: 16,
            y: 16,
            w: 64,
            h: 48,
            color: 0x0000FF00,
        };
        let rect_bytes = rect.to_bytes();
        core::ptr::copy_nonoverlapping(
            rect_bytes.as_ptr(),
            (data_va + GFX_RECT_OFF) as *mut u8,
            rect_bytes.len(),
        );
        let header = crate::graphics::abi::GfxBlit {
            x: 100,
            y: 100,
            w: 2,
            h: 2,
        };
        let header_bytes = header.to_bytes();
        core::ptr::copy_nonoverlapping(
            header_bytes.as_ptr(),
            (data_va + GFX_BLIT_OFF) as *mut u8,
            header_bytes.len(),
        );
        let pixels: [u32; 4] = [0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF];
        for (index, pixel) in pixels.iter().enumerate() {
            let target = (data_va + GFX_PIX_OFF + (index as u64) * 4) as *mut u32;
            core::ptr::write_volatile(target, *pixel);
        }
    }
    GFX_MARKER_PHYS.store(data_phys, Ordering::Relaxed);
    let user_tid = unsafe { (*crate::task::process_manager()).allocate_thread_id() };
    let monitor_tid = unsafe { (*crate::task::process_manager()).allocate_thread_id() };
    let user_thread = Thread::new_user(user_tid, cpu, pid, GFX_CODE_ADDR, user_rsp);
    let monitor = Thread::new(monitor_tid, cpu, 0, gfx_monitor);
    unsafe {
        if !crate::task::SCHEDULERS[cpu].add_thread(user_thread, cpu) {
            crate::log_test!("GFX TEST: add user thread FAILED");
            return false;
        }
        (*crate::task::process_manager()).add_thread(pid, user_tid);
        if !crate::task::SCHEDULERS[cpu].add_thread(monitor, cpu) {
            crate::log_test!("GFX TEST: add monitor FAILED");
            return false;
        }
        (*crate::task::process_manager()).add_thread(0, monitor_tid);
    }
    crate::log_boot_raw!("GFX TEST: queued PID=");
    crate::log_boot_raw!("{}", pid);
    crate::log_boot_raw!(" CPU=");
    crate::log_boot_raw!("{}", cpu as u64);
    crate::log_boot!("");
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

    crate::log_ring3!("RING3 monitor started");

    loop {
        match marker_value() {
            Some(value) if value == USER_MARKER => {
                if !PASS_PRINTED.swap(true, Ordering::SeqCst) {
                    unsafe {
                        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
                    }

                    crate::log_exec!("User code executed (marker observed)");
                    crate::log_exec!("Kernel verified user execution marker");
                    crate::log_sched!("Returning/scheduling safely");
                    crate::log_vfs!("RING 3 USER THREAD TEST: PASS");

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
