use core::sync::atomic::{AtomicU64, Ordering};

use crate::drivers::storage::ide::IdeDevice;
use crate::exec::elf::{ExecError, ExecResult, ElfExecutor};
use crate::filesystem::vfs::Vfs;
use crate::memory::physical::FrameAllocator;
use crate::task::thread::Thread;

pub const HELLO_PATH: &[u8] = b"/bin/hello";
pub const HELLO_CPU: usize = 1;
pub const INIT_PATH: &[u8] = b"/bin/init";
pub const INIT_CPU: usize = 2;
pub const BADPTR_PATH: &[u8] = b"/bin/badptr";
pub const BADPTR_CPU: usize = 1;
const MAX_EXEC_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub enum VfsExecError {
    Superblock,
    GroupDescriptor,
    Open,
    Empty,
    TooLarge,
    Read,
    BadMagic,
    Exec(ExecError),
    CpuOffline,
}

/// PID of the VFS-executed processes, observed by their reaper threads.
static REAP_HELLO_PID: AtomicU64 = AtomicU64::new(0);
static REAP_INIT_PID: AtomicU64 = AtomicU64::new(0);
static REAP_BADPTR_PID: AtomicU64 = AtomicU64::new(0);

fn open_root_vfs() -> Result<(IdeDevice, Vfs), VfsExecError> {
    let mut ide = IdeDevice::new();

    let mut sb_buffer = [0u8; 512];
    if ide.read_sector(2, &mut sb_buffer).is_err() {
        return Err(VfsExecError::Superblock);
    }
    let sb = crate::filesystem::ext2::parse_superblock(&sb_buffer)
        .ok_or(VfsExecError::Superblock)?;

    let mut gd_buffer = [0u8; 512];
    if ide.read_sector(4, &mut gd_buffer).is_err() {
        return Err(VfsExecError::GroupDescriptor);
    }
    let gd = crate::filesystem::ext2::parse_group_descriptor(&gd_buffer)
        .ok_or(VfsExecError::GroupDescriptor)?;

    Ok((ide, Vfs::new(gd.inode_table_block(), 256, sb)))
}

fn read_path(
    vfs: &Vfs,
    ide: &mut IdeDevice,
    path: &[u8],
) -> Result<alloc::vec::Vec<u8>, VfsExecError> {
    let mut file = vfs.open(ide, path).ok_or(VfsExecError::Open)?;

    let size = file.inode.size as usize;
    if size == 0 {
        return Err(VfsExecError::Empty);
    }
    if size > MAX_EXEC_BYTES {
        return Err(VfsExecError::TooLarge);
    }

    let mut data = alloc::vec![0u8; size];
    let bytes_read = vfs.read(ide, &mut file, &mut data);
    if bytes_read != size {
        return Err(VfsExecError::Read);
    }

    if data.len() < 4
        || data[0] != 0x7F
        || data[1] != b'E'
        || data[2] != b'L'
        || data[3] != b'F'
    {
        return Err(VfsExecError::BadMagic);
    }

    Ok(data)
}

/// `exec(path, argv)`: VFS open -> read ELF bytes -> real ELF parse/load ->
/// user process/address space/user stack/argv -> user thread on `cpu_id`.
pub fn exec_path(
    path: &[u8],
    argv: &[&str],
    cpu_id: usize,
) -> Result<ExecResult, VfsExecError> {
    crate::drivers::serial::print("VFS EXEC: open ");
    for &b in path {
        crate::drivers::serial::putc(b);
    }
    crate::drivers::serial::println("");

    if !crate::arch::x86_64::smp::percpu::is_online(cpu_id) {
        crate::drivers::serial::println("VFS EXEC: target CPU offline");
        return Err(VfsExecError::CpuOffline);
    }

    let (mut ide, vfs) = open_root_vfs()?;
    let elf = read_path(&vfs, &mut ide, path)?;

    crate::drivers::serial::print("VFS EXEC: ELF bytes=");
    crate::drivers::serial::print_u64(elf.len() as u64);
    crate::drivers::serial::println(" magic OK");

    let mut allocator = FrameAllocator::new();
    let pm_ptr = crate::task::process_manager();

    let res = unsafe {
        ElfExecutor::exec_with_argv_on_cpu(&elf, argv, &mut *pm_ptr, &mut allocator, cpu_id)
    }
    .map_err(VfsExecError::Exec)?;

    crate::drivers::serial::print("VFS EXEC: PROCESS CREATED PID=");
    crate::drivers::serial::print_u64(res.process_id);
    crate::drivers::serial::print(" TID=");
    crate::drivers::serial::print_u64(res.thread_id);
    crate::drivers::serial::print(" entry=0x");
    crate::drivers::serial::print_hex_u64(res.entry);
    crate::drivers::serial::print(" RSP=0x");
    crate::drivers::serial::print_hex_u64(res.user_stack_pointer);
    crate::drivers::serial::println("");

    Ok(res)
}

/// Kernel reaper for the VFS-executed hello: waits until the process is
/// terminated with no live threads, then reaps it and exits itself.
extern "C" fn hello_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::serial::println("VFS EXEC: hello reaper started");

    loop {
        let pid = REAP_HELLO_PID.load(Ordering::Relaxed);
        if pid != 0 {
            let ready = unsafe {
                match (*crate::task::process_manager()).get(pid) {
                    Some(_) => {
                        let pm = crate::task::process_manager();
                        (*pm).get(pid).map(|p| p.is_terminated()).unwrap_or(false)
                            && !(*pm).has_live_threads(pid)
                    }
                    None => false,
                }
            };
            if ready {
                crate::drivers::serial::print("VFS EXEC: hello PID=");
                crate::drivers::serial::print_u64(pid);
                crate::drivers::serial::println(" exited, reaping...");

                let mut allocator = FrameAllocator::new();
                let ok =
                    unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };
                let gone =
                    unsafe { (*crate::task::process_manager()).get(pid).is_none() };
                if ok && gone {
                    crate::drivers::serial::println("VFS EXEC: hello PROCESS REAP OK");
                    crate::task::exit();
                } else {
                    crate::drivers::serial::println("VFS EXEC: hello reap FAILED");
                }
            }
        }
        crate::task::sleep(20);
    }
}

extern "C" fn init_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::serial::println("VFS EXEC: init reaper started");

    loop {
        let pid = REAP_INIT_PID.load(Ordering::Relaxed);
        if pid != 0 {
            let ready = unsafe {
                match (*crate::task::process_manager()).get(pid) {
                    Some(_) => {
                        let pm = crate::task::process_manager();
                        (*pm).get(pid).map(|p| p.is_terminated()).unwrap_or(false)
                            && !(*pm).has_live_threads(pid)
                    }
                    None => false,
                }
            };
            if ready {
                crate::drivers::serial::print("VFS EXEC: init PID=");
                crate::drivers::serial::print_u64(pid);
                crate::drivers::serial::println(" exited, reaping...");

                let mut allocator = FrameAllocator::new();
                let ok =
                    unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };
                let gone =
                    unsafe { (*crate::task::process_manager()).get(pid).is_none() };
                if ok && gone {
                    crate::drivers::serial::println("VFS EXEC: init PROCESS REAP OK");
                    crate::task::exit();
                } else {
                    crate::drivers::serial::println("VFS EXEC: init reap FAILED");
                }
            }
        }
        crate::task::sleep(20);
    }
}

extern "C" fn badptr_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }
    crate::drivers::serial::println("VFS EXEC: badptr reaper started");
    loop {
        let pid = REAP_BADPTR_PID.load(Ordering::Relaxed);
        if pid != 0 {
            let ready = unsafe {
                match (*crate::task::process_manager()).get(pid) {
                    Some(_) => {
                        let pm = crate::task::process_manager();
                        (*pm).get(pid).map(|p| p.is_terminated()).unwrap_or(false)
                            && !(*pm).has_live_threads(pid)
                    }
                    None => false,
                }
            };
            if ready {
                crate::drivers::serial::print("VFS EXEC: badptr PID=");
                crate::drivers::serial::print_u64(pid);
                crate::drivers::serial::println(" exited, reaping...");
                let mut allocator = FrameAllocator::new();
                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };
                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };
                if ok && gone {
                    crate::drivers::serial::println("VFS EXEC: badptr PROCESS REAP OK");
                    crate::task::exit();
                } else {
                    crate::drivers::serial::println("VFS EXEC: badptr reap FAILED");
                }
            }
        }
        crate::task::sleep(20);
    }
}

fn spawn_reaper(pid: u64, cpu_id: usize, kind: u8) -> bool {
    let tid = unsafe { (*crate::task::process_manager()).allocate_thread_id() };
    let func: extern "C" fn() -> ! = match kind {
        1 => init_reaper,
        2 => badptr_reaper,
        _ => hello_reaper,
    };
    let thread = Thread::new(tid, cpu_id, 0, func);

    let added = unsafe { crate::task::SCHEDULERS[cpu_id].add_thread(thread, cpu_id) };
    if !added {
        crate::drivers::serial::println("VFS EXEC: reaper add_thread FAILED");
        return false;
    }
    unsafe {
        (*crate::task::process_manager()).add_thread(0, tid);
    }

    match kind {
        1 => REAP_INIT_PID.store(pid, Ordering::Relaxed),
        2 => REAP_BADPTR_PID.store(pid, Ordering::Relaxed),
        _ => REAP_HELLO_PID.store(pid, Ordering::Relaxed),
    }

    crate::drivers::serial::print("VFS EXEC: reaper TID=");
    crate::drivers::serial::print_u64(tid);
    match kind {
        1 => crate::drivers::serial::println(" armed for init"),
        2 => crate::drivers::serial::println(" armed for badptr"),
        _ => crate::drivers::serial::println(" armed for hello"),
    }

    true
}

/// Boot-time test: exec the real `/bin/hello` from VFS and arm its reaper.
/// Queues both threads on CPU1 (CPU0's scheduler is full at this stage).
/// Existing tests are untouched.
pub fn test_hello_exec() -> bool {
    crate::drivers::serial::println("=== VFS EXEC HELLO TEST ===");

    let argv: &[&str] = &["hello"];
    let res = match exec_path(HELLO_PATH, argv, HELLO_CPU) {
        Ok(r) => r,
        Err(e) => {
            crate::drivers::serial::print("VFS EXEC FAILED: ");
            crate::drivers::serial::println(alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::drivers::serial::println("VFS EXEC: unexpected entry FAILED");
        return false;
    }
    crate::drivers::serial::println("VFS EXEC: ELF LOAD OK");

    if !spawn_reaper(res.process_id, HELLO_CPU, 0) {
        crate::drivers::serial::println("VFS EXEC: reaper spawn FAILED");
        return false;
    }

    crate::drivers::serial::println("VFS EXEC: /bin/hello queued for Ring3");
    true
}

pub fn test_init_exec() -> bool {
    crate::drivers::serial::println("=== VFS EXEC INIT TEST ===");
    // INIT is the boot process: VFS -> Ring3 -> write -> exit -> reap
    let cpu = if crate::arch::x86_64::smp::percpu::is_online(INIT_CPU) {
        INIT_CPU
    } else {
        HELLO_CPU
    };
    let argv: &[&str] = &["init"];
    let res = match exec_path(INIT_PATH, argv, cpu) {
        Ok(r) => r,
        Err(e) => {
            crate::drivers::serial::print("VFS EXEC INIT FAILED: ");
            crate::drivers::serial::println(alloc::format!("{:?}", e).as_str());
            return false;
        }
    };
    if res.entry != 0x401000 {
        crate::drivers::serial::println("VFS EXEC INIT: unexpected entry FAILED");
        return false;
    }
    crate::drivers::serial::println("VFS EXEC INIT: ELF LOAD OK");
    if !spawn_reaper(res.process_id, cpu, 1) {
        crate::drivers::serial::println("VFS EXEC INIT: reaper spawn FAILED");
        return false;
    }
    crate::drivers::serial::println("VFS EXEC: /bin/init queued for Ring3");
    true
}

pub fn test_badptr_exec() -> bool {
    crate::drivers::serial::println("=== VFS EXEC BADPTR TEST ===");
    let cpu = if crate::arch::x86_64::smp::percpu::is_online(BADPTR_CPU) {
        BADPTR_CPU
    } else {
        HELLO_CPU
    };
    let argv: &[&str] = &["badptr"];
    let res = match exec_path(BADPTR_PATH, argv, cpu) {
        Ok(r) => r,
        Err(e) => {
            crate::drivers::serial::print("VFS EXEC BADPTR FAILED: ");
            crate::drivers::serial::println(alloc::format!("{:?}", e).as_str());
            return false;
        }
    };
    if res.entry != 0x401000 {
        crate::drivers::serial::println("VFS EXEC BADPTR: unexpected entry FAILED");
        return false;
    }
    crate::drivers::serial::println("VFS EXEC BADPTR: ELF LOAD OK");
    if !spawn_reaper(res.process_id, cpu, 2) {
        crate::drivers::serial::println("VFS EXEC BADPTR: reaper spawn FAILED");
        return false;
    }
    crate::drivers::serial::println("VFS EXEC: /bin/badptr queued for Ring3");
    true
}
