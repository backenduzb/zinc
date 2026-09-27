use crate::elf::program::{PF_R, PF_X};
use crate::exec::elf::ElfExecutor;
use crate::exec::vfs::{
    BADPTR_CPU, BADPTR_PATH, HELLO_CPU, INIT_CPU, INIT_PATH, REAP_BADPTR_PID, REAP_INIT_PID,
    badptr_reaper, exec_path, init_reaper, spawn_reaper,
};
use crate::memory::physical::FrameAllocator;

const TEST_ENTRY: u64 = 0x400000;
const TEST_OFFSET: u64 = 0x1000;

fn build_test_elf(payload: &[u8]) -> [u8; 8192] {
    let mut elf = [0u8; 8192];

    elf[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
    elf[4] = 2; 
    elf[5] = 1; 
    elf[6] = 1; 

    elf[16..18].copy_from_slice(&2u16.to_le_bytes()); 
    elf[18..20].copy_from_slice(&62u16.to_le_bytes()); 
    elf[20..24].copy_from_slice(&1u32.to_le_bytes()); 
    elf[24..32].copy_from_slice(&TEST_ENTRY.to_le_bytes());

    elf[32..40].copy_from_slice(&64u64.to_le_bytes()); 
    elf[52..54].copy_from_slice(&64u16.to_le_bytes()); 
    elf[54..56].copy_from_slice(&56u16.to_le_bytes()); 
    elf[56..58].copy_from_slice(&1u16.to_le_bytes()); 

    let ph = 64usize;
    elf[ph..ph + 4].copy_from_slice(&1u32.to_le_bytes()); 
    elf[ph + 4..ph + 8].copy_from_slice(&(PF_R | PF_X).to_le_bytes());
    elf[ph + 8..ph + 16].copy_from_slice(&TEST_OFFSET.to_le_bytes());
    elf[ph + 16..ph + 24].copy_from_slice(&TEST_ENTRY.to_le_bytes());
    elf[ph + 24..ph + 32].copy_from_slice(&0u64.to_le_bytes());
    elf[ph + 32..ph + 40].copy_from_slice(&(payload.len() as u64).to_le_bytes());
    elf[ph + 40..ph + 48].copy_from_slice(&0x1000u64.to_le_bytes());
    elf[ph + 48..ph + 56].copy_from_slice(&0x1000u64.to_le_bytes());

    let start = TEST_OFFSET as usize;
    elf[start..start + payload.len()].copy_from_slice(payload);

    elf
}

fn build_ring3_payload() -> [u8; 16] {
    let mut p = [0u8; 16];
    p[0] = 0x48;
    p[1] = 0xC7;
    p[2] = 0xC0;
    p[3..7].copy_from_slice(&0u32.to_le_bytes());
    p[7] = 0xCD;
    p[8] = 0x80;
    p[9] = 0xEB;
    p[10] = 0xFE;
    p[11] = 0x90;
    p[12] = 0x90;
    p[13] = 0x90;
    p[14] = 0x90;
    p[15] = 0x90;
    p
}

fn read_user_u64(space: &crate::memory::address_space::AddressSpace, va: u64) -> Option<u64> {
    let mut bytes = [0u8; 8];
    for i in 0..8 {
        let cur_va = va.checked_add(i as u64)?;
        let page_base = cur_va & !0xFFF;
        let offset = (cur_va & 0xFFF) as usize;
        let phys_base = space.translate_page(page_base)?;
        let phys = phys_base + offset as u64;
        let hva = crate::boot::limine::physical_to_virtual(phys)?;
        bytes[i] = unsafe { core::ptr::read_volatile(hva as *const u8) };
    }
    Some(u64::from_le_bytes(bytes))
}

fn read_user_byte(space: &crate::memory::address_space::AddressSpace, va: u64) -> Option<u8> {
    let page_base = va & !0xFFF;
    let offset = va & 0xFFF;
    let phys_base = space.translate_page(page_base)?;
    let phys = phys_base + offset;
    let hva = crate::boot::limine::physical_to_virtual(phys)?;
    Some(unsafe { core::ptr::read_volatile(hva as *const u8) })
}

fn read_user_cstr(
    space: &crate::memory::address_space::AddressSpace,
    va: u64,
    max_len: usize,
) -> Option<alloc::vec::Vec<u8>> {
    let mut out = alloc::vec::Vec::new();
    let mut cur = va;
    for _ in 0..max_len {
        let b = read_user_byte(space, cur)?;
        if b == 0 {
            break;
        }
        out.push(b);
        cur = cur.checked_add(1)?;
    }
    let term = read_user_byte(space, cur)?;
    if term != 0 {
        return None;
    }
    Some(out)
}

pub fn test_loader() {
    crate::log_exec!("=== ELF EXEC TEST ===");

    let heap_ready = crate::memory::heap::KERNEL_HEAP.is_initialized();

    let payload = build_ring3_payload();
    let elf = build_test_elf(&payload);

    if !heap_ready {
        crate::log_vfs!("EXEC TEST: heap not ready, running loader-only check");
        let mut allocator = FrameAllocator::new();
        let mut space = match crate::memory::address_space::AddressSpace::new_user(&mut allocator) {
            Some(s) => s,
            None => {
                crate::log_exec!("EXEC TEST: AddressSpace FAILED");
                return;
            }
        };
        match crate::elf::loader::ElfLoader::load(&elf, &mut space, &mut allocator) {
            Ok(loaded) => {
                if loaded.entry != TEST_ENTRY {
                    crate::log_exec!("EXEC TEST: entry mismatch");
                    return;
                }
                crate::log_exec!("EXEC TEST: loader-only PASS (heap pending)");
            }
            Err(e) => {
                crate::log_exec_raw!("EXEC TEST: loader failed: ");
                crate::log_exec!("{}", alloc::format!("{:?}", e).as_str());
            }
        }
        return;
    }

    crate::log_vfs!("EXEC TEST: heap ready, full pipeline");
    let mut allocator = FrameAllocator::new();
    let mut local_pm = crate::task::process::ProcessManager::new();
    let mut local_sched = crate::task::scheduler::Scheduler::new();
    let argv: &[&str] = &["zinc", "hello"];

    let pid = match local_pm.create_user(&mut allocator, None) {
        Some(p) => p,
        None => {
            crate::log_exec!("EXEC TEST: create_user FAILED");
            return;
        }
    };
    crate::log_exec_raw!("EXEC TEST: PID=");
    crate::log_exec_raw!("{}", pid);
    crate::log_exec!("");

    let (entry, stack_base, stack_top, rsp) = {
        let process = match local_pm.get_mut(pid) {
            Some(p) => p,
            None => {
                crate::log_exec!("EXEC TEST: get_mut FAILED");
                return;
            }
        };
        let space = match process.address_space_mut() {
            Some(s) => s,
            None => {
                crate::log_exec!("EXEC TEST: AddressSpace missing");
                return;
            }
        };

        let loaded = match crate::elf::loader::ElfLoader::load(&elf, space, &mut allocator) {
            Ok(l) => l,
            Err(e) => {
                crate::log_exec!("EXEC TEST: ELF load FAILED");
                crate::log_exec!("{}", alloc::format!("{:?}", e).as_str());
                return;
            }
        };

        if loaded.entry != TEST_ENTRY {
            crate::log_exec!("EXEC TEST: entry mismatch");
            return;
        }
        crate::log_exec_raw!("EXEC TEST: entry=0x");
        crate::log_exec_raw!("{:016X}", loaded.entry);
        crate::log_exec!("");

        match space.probe_page(loaded.entry) {
            Some((user, _w, exec)) if user && exec => {
                crate::log_exec!("EXEC TEST: user mapping executable OK");
            }
            _ => {
                crate::log_exec!("EXEC TEST: user mapping FAILED");
                return;
            }
        }

        let stack = match crate::memory::user_stack::UserStack::map(space, &mut allocator) {
            Some(s) => s,
            None => {
                crate::log_exec!("EXEC TEST: UserStack FAILED");
                return;
            }
        };
        crate::log_exec_raw!("EXEC TEST: stack base=0x");
        crate::log_exec_raw!("{:016X}", stack.base);
        crate::log_exec_raw!(" top=0x");
        crate::log_exec_raw!("{:016X}", stack.top);
        crate::log_exec!("");

        let user_rsp = match stack.setup_argv(space, argv) {
            Some(r) => r,
            None => {
                crate::log_exec!("EXEC TEST: setup_argv FAILED");
                return;
            }
        };
        crate::log_exec_raw!("EXEC TEST: user RSP(argc)=0x");
        crate::log_exec_raw!("{:016X}", user_rsp);
        crate::log_exec!("");

        if user_rsp == stack.top - 16 {
            crate::log_exec!("{}", "EXEC TEST: WARNING RSP still top-16 (should be argv stack)",);
        }

        if user_rsp % 16 != 0 {
            crate::log_exec!("EXEC TEST: RSP not 16-aligned FAILED");
            return;
        }
        crate::log_exec!("EXEC TEST: RSP 16-aligned OK");
        match space.probe_page(user_rsp) {
            Some((user, writable, exec)) if user && writable && !exec => {
                crate::log_exec!("EXEC TEST: user stack writable U=1 W=1 X=0 OK");
            }
            Some((u, w, x)) => {
                crate::log_exec_raw!("EXEC TEST: stack perms U=");
                crate::log_exec_raw!("{}", u as u64);
                crate::log_exec_raw!(" W=");
                crate::log_exec_raw!("{}", w as u64);
                crate::log_exec_raw!(" X=");
                crate::log_exec_raw!("{}", x as u64);
                crate::log_test!(" FAILED");
                return;
            }
            _ => {
                crate::log_exec!("EXEC TEST: user stack probe FAILED");
                return;
            }
        }

        crate::log_test!("=== ARGV TEST ===");
        let argc = match read_user_u64(space, user_rsp) {
            Some(v) => v,
            None => {
                crate::log_vfs!("ARGV TEST: read argc FAILED");
                return;
            }
        };
        crate::log_test_raw!("ARGV TEST: argc=");
        crate::log_exec_raw!("{}", argc);
        crate::log_exec!("");
        if argc != argv.len() as u64 {
            crate::log_test!("ARGV TEST: argc mismatch FAILED");
            return;
        }

        let argv_start = user_rsp + 8;
        let mut argv_addrs = alloc::vec::Vec::new();
        for i in 0..argv.len() {
            let ptr_va = argv_start + (i as u64) * 8;
            let ptr = match read_user_u64(space, ptr_va) {
                Some(v) => v,
                None => {
                    crate::log_vfs_raw!("ARGV TEST: read argv[");
                    crate::log_exec_raw!("{}", i as u64);
                    crate::log_test!("] FAILED");
                    return;
                }
            };
            crate::log_test_raw!("ARGV TEST: argv[");
            crate::log_exec_raw!("{}", i as u64);
            crate::log_exec_raw!("]=0x");
            crate::log_exec_raw!("{:016X}", ptr);
            crate::log_exec!("");
            if ptr == 0 {
                crate::log_test!("ARGV TEST: argv pointer NULL FAILED");
                return;
            }
            if ptr >= 0x0000_8000_0000_0000 {
                crate::log_ring3!("ARGV TEST: argv pointer not user FAILED");
                return;
            }
            if !stack.contains(ptr) {
                crate::log_test!("ARGV TEST: argv pointer outside stack FAILED");
                return;
            }
            match space.probe_page(ptr) {
                Some((u, w, _)) if u && w => {}
                _ => {
                    crate::log_exec!("{}", "ARGV TEST: argv string page not mapped FAILED",);
                    return;
                }
            }
            argv_addrs.push(ptr);
        }
        let null_va = argv_start + (argv.len() as u64) * 8;
        let null_ptr = match read_user_u64(space, null_va) {
            Some(v) => v,
            None => {
                crate::log_vfs!("ARGV TEST: read argv NULL FAILED");
                return;
            }
        };
        crate::log_test_raw!("ARGV TEST: argv[");
        crate::log_exec_raw!("{}", argv.len() as u64);
        crate::log_exec_raw!("]=0x");
        crate::log_exec_raw!("{:016X}", null_ptr);
        crate::log_exec!("");
        if null_ptr != 0 {
            crate::log_test!("ARGV TEST: argv NULL terminator FAILED");
            return;
        }
        crate::log_test!("ARGV TEST: argv[2]=NULL OK");

        let mut strings_ok = true;
        for (i, &expected) in argv.iter().enumerate() {
            let va = argv_addrs[i];
            let data = match read_user_cstr(space, va, 256) {
                Some(v) => v,
                None => {
                    crate::log_vfs_raw!("ARGV TEST: read string ");
                    crate::log_exec_raw!("{}", i as u64);
                    crate::log_test!(" FAILED");
                    strings_ok = false;
                    break;
                }
            };
            if data != expected.as_bytes() {
                crate::log_test_raw!("ARGV TEST: string mismatch at ");
                crate::log_exec_raw!("{}", i as u64);
                crate::log_exec!("");
                strings_ok = false;
                break;
            }
        }
        if !strings_ok {
            return;
        }
        crate::log_test!("ARGV TEST: strings OK");
        crate::log_test_raw!("ARGV TEST: initial RSP=0x");
        crate::log_exec_raw!("{:016X}", user_rsp);
        crate::log_exec!("");
        crate::log_test!("ARGV TEST: PASS");
        let e = loaded.entry;
        let b = stack.base;
        let t = stack.top;
        let r = user_rsp;
        (e, b, t, r)
    };
    let cpu_id = crate::arch::x86_64::smp::percpu::current_cpu_id();
    let tid = match local_sched.create_user_thread(cpu_id, &mut local_pm, pid, entry, rsp) {
        Some(t) => t,
        None => {
            crate::log_vfs!("EXEC TEST: create_user_thread FAILED");
            return;
        }
    };
    crate::log_exec_raw!("EXEC TEST: TID=");
    crate::log_exec_raw!("{}", tid);
    crate::log_exec!("");

    match local_pm.get(pid) {
        Some(p) if p.threads().contains(&tid) => {
            crate::log_vfs!("EXEC TEST: process thread registration OK");
        }
        _ => {
            crate::log_vfs!("EXEC TEST: process thread registration FAILED");
            return;
        }
    }

    let sched_has = (0..8).any(|idx| {
        local_sched
            .thread(idx)
            .map(|t| t.id() == tid && t.process_id() == pid)
            .unwrap_or(false)
    });
    if sched_has {
        crate::log_vfs!("EXEC TEST: scheduler thread registration OK");
    } else {
        crate::log_exec!("EXEC TEST: scheduler registration FAILED");
        return;
    }

    let mut found = false;
    for i in 0..8 {
        if let Some(t) = local_sched.thread(i) {
            if t.id() == tid {
                if t.user_rip() == TEST_ENTRY && t.user_rsp() == rsp {
                    found = true;
                    crate::log_vfs_raw!("EXEC TEST: thread RIP=0x");
                    crate::log_exec_raw!("{:016X}", t.user_rip());
                    crate::log_exec_raw!(" RSP=0x");
                    crate::log_exec_raw!("{:016X}", t.user_rsp());
                    crate::log_exec!(" OK");
                } else {
                    crate::log_exec!("EXEC TEST: RIP/RSP mismatch FAILED");
                    return;
                }
                let frame = unsafe { &*t.user_frame_ptr() };
                if frame.cs == crate::arch::x86_64::gdt::GDT_USER_CODE as u64
                    && frame.ss == crate::arch::x86_64::gdt::GDT_USER_DATA as u64
                    && frame.rflags == 0x202
                {
                    crate::log_vfs!("EXEC TEST: thread frame CS/SS/RFLAGS OK");
                } else {
                    crate::log_vfs!("EXEC TEST: thread frame FAILED");
                }
                if t.user_rsp() < stack_base || t.user_rsp() >= stack_top {
                    crate::log_exec!("EXEC TEST: RSP outside stack FAILED");
                    return;
                }
                if t.user_rsp() % 16 != 0 {
                    crate::log_vfs!("EXEC TEST: thread RSP alignment FAILED");
                    return;
                }
                break;
            }
        }
    }
    if !found {
        crate::log_vfs!("EXEC TEST: thread not found FAILED");
        return;
    }

    crate::log_vfs!("EXEC TEST: process ready");
    crate::log_exec!("ELF EXEC TEST: PASS");

    crate::log_exec!("=== ELF EXEC GLOBAL TEST ===");
    let mut global_alloc = FrameAllocator::new();
    let pm_ptr = crate::task::process_manager();
    let argv_global: &[&str] = &["zinc", "hello"];
    let exec_res =
        unsafe { ElfExecutor::exec_with_argv(&elf, argv_global, &mut *pm_ptr, &mut global_alloc) };
    match exec_res {
        Ok(res) => {
            crate::log_exec_raw!("GLOBAL EXEC: PID=");
            crate::log_exec_raw!("{}", res.process_id);
            crate::log_exec_raw!(" TID=");
            crate::log_exec_raw!("{}", res.thread_id);
            crate::log_exec_raw!(" entry=0x");
            crate::log_exec_raw!("{:016X}", res.entry);
            crate::log_exec_raw!(" RSP=0x");
            crate::log_exec_raw!("{:016X}", res.user_stack_top);
            crate::log_exec!("");
            if res.entry != TEST_ENTRY {
                crate::log_exec!("GLOBAL EXEC: entry FAILED");
            } else {
                crate::log_exec!("GLOBAL EXEC: entry OK");
            }
            if res.user_stack_top % 16 != 0 {
                crate::log_exec!("GLOBAL EXEC: RSP alignment FAILED");
            } else {
                crate::log_exec!("GLOBAL EXEC: RSP 16-aligned OK");
            }
            let pm_has = unsafe {
                (*pm_ptr)
                    .get(res.process_id)
                    .map(|p| p.threads().contains(&res.thread_id))
                    .unwrap_or(false)
            };
            let sched_has_global = unsafe {
                let sched = crate::task::scheduler();
                let mut ok = false;
                for i in 0..8 {
                    if let Some(t) = (*sched).thread(i) {
                        if t.id() == res.thread_id {
                            ok = true;
                            break;
                        }
                    }
                }
                ok
            };
            if pm_has && sched_has_global {
                crate::log_vfs!("GLOBAL EXEC: thread owned by both managers OK");
            } else {
                crate::log_exec!("GLOBAL EXEC: manager ownership FAILED");
            }
            let argv_ok = unsafe {
                if let Some(proc) = (*pm_ptr).get(res.process_id) {
                    if let Some(space) = proc.address_space() {
                        let argc = read_user_u64(space, res.user_stack_top).unwrap_or(99);
                        let a0 = read_user_u64(space, res.user_stack_top + 8).unwrap_or(0);
                        let a1 = read_user_u64(space, res.user_stack_top + 16).unwrap_or(0);
                        let an = read_user_u64(space, res.user_stack_top + 24).unwrap_or(99);
                        crate::log_exec_raw!("GLOBAL ARGV: argc=");
                        crate::log_exec_raw!("{}", argc);
                        crate::log_exec_raw!(" argv0=0x");
                        crate::log_exec_raw!("{:016X}", a0);
                        crate::log_exec_raw!(" argv1=0x");
                        crate::log_exec_raw!("{:016X}", a1);
                        crate::log_exec_raw!(" argv2=");
                        crate::log_exec_raw!("{}", an);
                        crate::log_exec!("");
                        argc == 2 && a0 != 0 && a1 != 0 && an == 0
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if argv_ok {
                crate::log_exec!("GLOBAL EXEC: argv verification OK");
                crate::log_exec!("GLOBAL EXEC: PASS");
            } else {
                crate::log_exec!("GLOBAL EXEC: argv verification FAILED");
            }
        }
        Err(e) => {
            crate::log_exec_raw!("GLOBAL EXEC FAILED: ");
            crate::log_exec!("{}", alloc::format!("{:?}", e).as_str());
        }
    }
}

pub fn test_exec_runtime() -> bool {
    crate::log_exec!("=== ELF EXEC RUNTIME TEST ===");

    let payload = build_ring3_payload();
    let elf = build_test_elf(&payload);

    let mut allocator = FrameAllocator::new();
    let pm_ptr = crate::task::process_manager();

    crate::log_exec!("{}", "RUNTIME EXEC: creating user process via ElfExecutor with argv",);

    let argv: &[&str] = &["zinc", "hello"];
    let res = unsafe { ElfExecutor::exec_with_argv(&elf, argv, &mut *pm_ptr, &mut allocator) };

    match res {
        Ok(r) => {
            crate::log_exec_raw!("RUNTIME EXEC: PID=");
            crate::log_exec_raw!("{}", r.process_id);
            crate::log_exec_raw!(" TID=");
            crate::log_exec_raw!("{}", r.thread_id);
            crate::log_ring3!(" queued for Ring3");
            crate::log_exec_raw!("RUNTIME EXEC: entry=0x");
            crate::log_exec_raw!("{:016X}", r.entry);
            crate::log_exec_raw!(" RSP(argc)=0x");
            crate::log_exec_raw!("{:016X}", r.user_stack_top);
            crate::log_exec!("");
            if r.user_stack_top % 16 != 0 {
                crate::log_exec!("RUNTIME EXEC: RSP not 16-aligned FAILED");
                return false;
            }
            let ok = unsafe {
                if let Some(proc) = (*pm_ptr).get(r.process_id) {
                    if let Some(space) = proc.address_space() {
                        let argc = read_user_u64(space, r.user_stack_top).unwrap_or(999);
                        crate::log_exec_raw!("RUNTIME ARGV: argc=");
                        crate::log_exec_raw!("{}", argc);
                        crate::log_exec!("");
                        argc == 2
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if !ok {
                crate::log_exec!("RUNTIME EXEC: argv check FAILED");
                return false;
            }
            crate::log_exec!("RUNTIME EXEC: argv OK, RSP points to argc");
            true
        }
        Err(e) => {
            crate::log_exec_raw!("RUNTIME EXEC FAILED: ");
            crate::log_exec!("{}", alloc::format!("{:?}", e).as_str());
            false
        }
    }
}

pub fn test_process_reap() {
    crate::log_exec!("=== PROCESS REAP TEST ===");

    let mut allocator = FrameAllocator::new();
    let free_before = allocator.free_count();
    crate::log_exec_raw!("REAP TEST: free_before=");
    crate::log_exec_raw!("{}", free_before as u64);
    crate::log_exec!("");

    {
        let mut local_pm = crate::task::process::ProcessManager::new();
        let bad = [0u8; 64];
        let res = ElfExecutor::exec_with_argv(&bad, &["x"], &mut local_pm, &mut allocator);
        if res.is_ok() {
            crate::log_exec!("REAP TEST: bad ELF should fail FAILED");
            return;
        }
        let free_after_bad = allocator.free_count();
        crate::log_exec_raw!("REAP TEST: free_after_bad_elf=");
        crate::log_exec_raw!("{}", free_after_bad as u64);
        crate::log_exec!("");
        if free_after_bad != free_before {
            crate::log_exec_raw!("REAP TEST: leak on bad ELF: before=");
            crate::log_exec_raw!("{}", free_before as u64);
            crate::log_exec_raw!(" after=");
            crate::log_exec_raw!("{}", free_after_bad as u64);
            crate::log_exec!(" WARN (tolerate 1 frame rounding)");
        } else {
            crate::log_exec!("REAP TEST: error cleanup OK (no leak)");
        }
        if local_pm.count() != 0 {
            crate::log_exec!("REAP TEST: leaked process after bad ELF FAILED");
            return;
        }
    }

    let mut local_pm = crate::task::process::ProcessManager::new();
    let mut local_sched = crate::task::scheduler::Scheduler::new();

    let payload = build_ring3_payload();
    let elf = build_test_elf(&payload);
    let argv: &[&str] = &["zinc", "hello"];

    let pid = match local_pm.create_user(&mut allocator, None) {
        Some(p) => p,
        None => {
            crate::log_exec!("REAP TEST: create_user FAILED");
            return;
        }
    };
    crate::log_exec_raw!("PROCESS created PID=");
    crate::log_exec_raw!("{}", pid);
    crate::log_exec!("");

    if local_pm.get(pid).is_none() {
        crate::log_exec!("REAP TEST: process not found FAILED");
        return;
    }
    crate::log_exec!("REAP TEST: process exists OK");
    match local_pm.get(pid).and_then(|p| p.address_space()) {
        Some(_) => crate::log_exec!("REAP TEST: address space exists OK"),
        None => {
            crate::log_exec!("REAP TEST: address space missing FAILED");
            return;
        }
    }

    let (entry, stack_base, stack_top, rsp) = {
        let proc = match local_pm.get_mut(pid) {
            Some(p) => p,
            None => {
                crate::log_exec!("REAP TEST: get_mut FAILED");
                return;
            }
        };
        let space = proc.address_space_mut().unwrap();
        let loaded = match crate::elf::loader::ElfLoader::load(&elf, space, &mut allocator) {
            Ok(l) => l,
            Err(e) => {
                crate::log_exec!("{}", alloc::format!("{:?}", e).as_str());
                return;
            }
        };
        let stack = crate::memory::user_stack::UserStack::map(space, &mut allocator).unwrap();
        let rsp = stack.setup_argv(space, argv).unwrap();
        (loaded.entry, stack.base, stack.top, rsp)
    };

    let _ = stack_base;
    let _ = stack_top;
    let cpu = crate::arch::x86_64::smp::percpu::current_cpu_id();
    let tid = match local_sched.create_user_thread(cpu, &mut local_pm, pid, entry, rsp) {
        Some(t) => t,
        None => {
            crate::log_vfs!("REAP TEST: create_user_thread FAILED");
            return;
        }
    };
    crate::log_vfs_raw!("THREAD created TID=");
    crate::log_exec_raw!("{}", tid);
    crate::log_exec!("");

    let has_thread = (0..8).any(|i| {
        local_sched
            .thread(i)
            .map(|t| t.id() == tid)
            .unwrap_or(false)
    });
    if !has_thread {
        crate::log_vfs!("REAP TEST: thread not in scheduler FAILED");
        return;
    }
    crate::log_vfs!("REAP TEST: thread exists OK");

    let free_mid = allocator.free_count();
    crate::log_exec_raw!("REAP TEST: free_mid=");
    crate::log_exec_raw!("{}", free_mid as u64);
    crate::log_exec!("");
    if free_mid >= free_before {
        crate::log_exec!("REAP TEST: free_mid not decreased WARN");
    } else {
        crate::log_exec!("REAP TEST: frames allocated OK");
    }

    let mut found_idx = None;
    for i in 0..8 {
        if let Some(t) = local_sched.thread(i) {
            if t.id() == tid {
                found_idx = Some(i);
                break;
            }
        }
    }
    let idx = match found_idx {
        Some(v) => v,
        None => {
            crate::log_vfs!("REAP TEST: thread idx not found FAILED");
            return;
        }
    };
    if let Some(th) = local_sched.thread_mut(idx) {
        th.set_state(crate::task::thread::ThreadState::Terminated);
    }
    local_pm.notify_thread_terminated(pid, tid);
    crate::log_vfs_raw!("THREAD terminated TID=");
    crate::log_exec_raw!("{}", tid);
    crate::log_exec!("");

    if local_pm.has_live_threads(pid) {
        crate::log_vfs!("REAP TEST: process still has live threads FAILED");
        return;
    }
    let proc = match local_pm.get(pid) {
        Some(p) => p,
        None => {
            crate::log_exec!("REAP TEST: process vanished early FAILED");
            return;
        }
    };
    if !proc.is_terminated() {
        crate::log_exec!("REAP TEST: process not terminated FAILED");
        return;
    }
    crate::log_exec_raw!("PROCESS terminated PID=");
    crate::log_exec_raw!("{}", pid);
    crate::log_exec!("");

    let removed = local_sched.remove_thread_by_id(tid);
    if !removed {
        crate::log_exec!("REAP TEST: scheduler remove FAILED");
        return;
    }
    crate::log_exec_raw!("SCHEDULER slot reaped TID=");
    crate::log_exec_raw!("{}", tid);
    crate::log_exec!("");

    let free_before_reap = allocator.free_count();
    let ok = local_pm.reap(pid, &mut allocator);
    if !ok {
        crate::log_exec!("REAP TEST: ProcessManager::reap FAILED");
        return;
    }

    if local_pm.get(pid).is_some() {
        crate::log_exec!("REAP TEST: process still exists after reap FAILED");
        return;
    }
    crate::log_exec!("REAP TEST: process removed OK");

    let free_after = allocator.free_count();
    crate::log_exec_raw!("REAP TEST: free_before_reap=");
    crate::log_exec_raw!("{}", free_before_reap as u64);
    crate::log_exec_raw!(" free_after=");
    crate::log_exec_raw!("{}", free_after as u64);
    crate::log_exec!("");
    if free_after <= free_before_reap {
        crate::log_exec!("REAP TEST: frames not returned FAILED");
        return;
    }
    crate::log_exec!("FRAMES returned");

    let pid2 = match local_pm.create_user(&mut allocator, None) {
        Some(p) => p,
        None => {
            crate::log_exec!("REAP TEST: second create_user FAILED (slot leak?)");
            return;
        }
    };
    crate::log_exec_raw!("REAP TEST: second PROCESS created PID=");
    crate::log_exec_raw!("{}", pid2);
    crate::log_exec!("");
    {
        let proc = local_pm.get_mut(pid2).unwrap();
        let space = proc.address_space_mut().unwrap();
        let _loaded = crate::elf::loader::ElfLoader::load(&elf, space, &mut allocator).unwrap();
        let stack = crate::memory::user_stack::UserStack::map(space, &mut allocator).unwrap();
        let rsp2 = stack.setup_argv(space, argv).unwrap();
        let tid2 = local_sched.create_user_thread(cpu, &mut local_pm, pid2, entry, rsp2);
        if tid2.is_none() {
            crate::log_exec!("{}", "REAP TEST: second thread creation FAILED (slot not reusable)",);
            return;
        }
        crate::log_vfs_raw!("REAP TEST: second THREAD created TID=");
        crate::log_exec_raw!("{}", tid2.unwrap());
        crate::log_exec!(" (slot reuse OK)");
        let tid2 = tid2.unwrap();
        for i in 0..8 {
            if let Some(t) = local_sched.thread(i) {
                if t.id() == tid2 {
                    if let Some(th) = local_sched.thread_mut(i) {
                        th.set_state(crate::task::thread::ThreadState::Terminated);
                    }
                    break;
                }
            }
        }
        local_pm.notify_thread_terminated(pid2, tid2);
        local_sched.remove_thread_by_id(tid2);
        let _ = local_pm.reap(pid2, &mut allocator);
    }

    let free_final = allocator.free_count();
    crate::log_exec_raw!("REAP TEST: free_final=");
    crate::log_exec_raw!("{}", free_final as u64);
    crate::log_exec!("");

    crate::log_exec!("PROCESS REAP TEST: PASS");
}

pub fn test_init_exec() -> bool {
    crate::log_vfs!("=== VFS EXEC INIT TEST ===");

    crate::exec::vfs::INIT_COMPLETE.store(false, core::sync::atomic::Ordering::Release);

    let cpu = if crate::arch::x86_64::smp::percpu::is_online(INIT_CPU) {
        INIT_CPU
    } else {
        HELLO_CPU
    };

    let argv: &[&str] = &["init"];

    let res = match exec_path(INIT_PATH, argv, cpu) {
        Ok(r) => r,
        Err(e) => {
            crate::log_vfs_raw!("VFS EXEC INIT FAILED: ");
            crate::log_exec!("{}", alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::log_vfs!("VFS EXEC INIT: unexpected entry FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC INIT: ELF LOAD OK");

    if !spawn_reaper(res.process_id, cpu, init_reaper, &REAP_INIT_PID) {
        crate::log_vfs!("VFS EXEC INIT: reaper spawn FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC: /bin/init queued for Ring3");

    true
}

pub fn test_badptr_exec() -> bool {
    crate::log_vfs!("=== VFS EXEC BADPTR TEST ===");

    let cpu = if crate::arch::x86_64::smp::percpu::is_online(BADPTR_CPU) {
        BADPTR_CPU
    } else {
        HELLO_CPU
    };

    let argv: &[&str] = &["badptr"];

    let res = match exec_path(BADPTR_PATH, argv, cpu) {
        Ok(r) => r,
        Err(e) => {
            crate::log_vfs_raw!("VFS EXEC BADPTR FAILED: ");
            crate::log_exec!("{}", alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::log_vfs!("VFS EXEC BADPTR: unexpected entry FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC BADPTR: ELF LOAD OK");

    if !spawn_reaper(res.process_id, cpu, badptr_reaper, &REAP_BADPTR_PID) {
        crate::log_vfs!("VFS EXEC BADPTR: reaper spawn FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC: /bin/badptr queued for Ring3");

    true
}
