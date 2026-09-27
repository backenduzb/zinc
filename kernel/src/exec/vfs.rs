use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::drivers::storage::ide::IdeDevice;
use crate::exec::elf::{ElfExecutor, ExecError, ExecResult};
use crate::filesystem::vfs::Vfs;
use crate::memory::physical::FrameAllocator;
use crate::task::thread::Thread;

pub const HELLO_PATH: &[u8] = b"/bin/hello";
pub const HELLO_CPU: usize = 1;

pub const INIT_PATH: &[u8] = b"/bin/init";
pub const INIT_CPU: usize = 2;

pub const BADPTR_PATH: &[u8] = b"/bin/badptr";
pub const BADPTR_CPU: usize = 3;

pub const READ_TEST_PATH: &[u8] = b"/bin/read_test";
pub const READ_TEST_CPU: usize = 3;

pub const OPEN_TEST_PATH: &[u8] = b"/bin/open_test";
pub const OPEN_TEST_CPU: usize = 3;

pub const EXECVE_TEST_PATH: &[u8] = b"/bin/exec_test";
pub const EXECVE_TEST_CPU: usize = 3;

pub const CLOSE_TEST_PATH: &[u8] = b"/bin/close_test";
pub const CLOSE_TEST_CPU: usize = 3;

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
    ThreadCreationFailed,
    ReaperSpawnFailed,
}

pub(crate) static REAP_PID: AtomicU64 = AtomicU64::new(0);
pub static REAP_INIT_PID: AtomicU64 = AtomicU64::new(0);
pub static REAP_BADPTR_PID: AtomicU64 = AtomicU64::new(0);
pub(crate) static REAP_OPEN_TEST_PID: AtomicU64 = AtomicU64::new(0);
pub(crate) static REAP_READ_TEST_PID: AtomicU64 = AtomicU64::new(0);
pub(crate) static REAP_CLOSE_TEST_PID: AtomicU64 = AtomicU64::new(0);
pub(crate) static REAP_EXECVE_TEST_PID: AtomicU64 = AtomicU64::new(0);

pub(crate) static BADPTR_COMPLETE: AtomicBool = AtomicBool::new(false);
pub(crate) static HELLO_COMPLETE: AtomicBool = AtomicBool::new(false);
pub(crate) static INIT_COMPLETE: AtomicBool = AtomicBool::new(false);
pub(crate) static OPEN_TEST_COMPLETE: AtomicBool = AtomicBool::new(false);
pub(crate) static READ_TEST_COMPLETE: AtomicBool = AtomicBool::new(false);
pub(crate) static CLOSE_TEST_COMPLETE: AtomicBool = AtomicBool::new(false);
pub(crate) static EXECVE_TEST_COMPLETE: AtomicBool = AtomicBool::new(false);

pub(crate) fn open_root_vfs() -> Result<(IdeDevice, Vfs), VfsExecError> {
    let mut ide = IdeDevice::new();

    let mut sb_buffer = [0u8; 512];
    if ide.read_sector(2, &mut sb_buffer).is_err() {
        return Err(VfsExecError::Superblock);
    }
    let sb =
        crate::filesystem::ext2::parse_superblock(&sb_buffer).ok_or(VfsExecError::Superblock)?;

    let mut gd_buffer = [0u8; 512];
    if ide.read_sector(4, &mut gd_buffer).is_err() {
        return Err(VfsExecError::GroupDescriptor);
    }
    let gd = crate::filesystem::ext2::parse_group_descriptor(&gd_buffer)
        .ok_or(VfsExecError::GroupDescriptor)?;

    Ok((
        ide,
        Vfs::new(
            gd.inode_table_block(),
            256,
            sb,
            gd.inode_bitmap,
            gd.block_bitmap,
        )
    ))
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

    if data.len() < 4 || data[0] != 0x7F || data[1] != b'E' || data[2] != b'L' || data[3] != b'F' {
        return Err(VfsExecError::BadMagic);
    }

    Ok(data)
}

pub fn read_exec_path(path: &[u8]) -> Result<Vec<u8>, VfsExecError> {
    let (mut ide, vfs) = open_root_vfs()?;

    let elf = read_path(&vfs, &mut ide, path)?;

    if elf.len() < 4 || &elf[..4] != b"\x7FELF" {
        return Err(VfsExecError::BadMagic);
    }

    crate::log_vfs_raw!("VFS EXEC: ELF bytes=");
    crate::log_vfs_raw!("{}", elf.len() as u64);
    crate::log_vfs!(" magic OK");

    Ok(elf)
}

pub fn exec_path(path: &[u8], argv: &[&str], cpu_id: usize) -> Result<ExecResult, VfsExecError> {
    crate::log_vfs_raw!("VFS EXEC: open ");
    for &b in path {
        crate::log_vfs_raw!("{}", (b) as char);
    }
    crate::log_vfs!("");

    if !crate::arch::x86_64::smp::percpu::is_online(cpu_id) {
        crate::log_vfs!("VFS EXEC: target CPU offline");
        return Err(VfsExecError::CpuOffline);
    }

    let (mut ide, vfs) = open_root_vfs()?;
    let elf = read_path(&vfs, &mut ide, path)?;

    crate::log_vfs_raw!("VFS EXEC: ELF bytes=");
    crate::log_vfs_raw!("{}", elf.len() as u64);
    crate::log_vfs!(" magic OK");

    let mut allocator = FrameAllocator::new();
    let pm_ptr = crate::task::process_manager();

    let res = unsafe {
        ElfExecutor::exec_with_argv_on_cpu(&elf, argv, &mut *pm_ptr, &mut allocator, cpu_id)
    }
    .map_err(VfsExecError::Exec)?;

    crate::log_vfs_raw!("VFS EXEC: PROCESS CREATED PID=");
    crate::log_vfs_raw!("{}", res.process_id);
    crate::log_vfs_raw!(" TID=");
    crate::log_vfs_raw!("{}", res.thread_id);
    crate::log_vfs_raw!(" entry=0x");
    crate::log_vfs_raw!("{:016X}", res.entry);
    crate::log_vfs_raw!(" RSP=0x");
    crate::log_vfs_raw!("{:016X}", res.user_stack_pointer);
    crate::log_vfs!("");

    Ok(res)
}

extern "C" fn hello_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::log_vfs!("VFS EXEC: reaper started");

    loop {
        let pid = REAP_PID.load(Ordering::Relaxed);
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
                crate::log_vfs_raw!("VFS EXEC: PID=");
                crate::log_vfs_raw!("{}", pid);
                crate::log_exec!(" exited, reaping...");

                let mut allocator = FrameAllocator::new();
                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };
                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };
                if ok && gone {
                    crate::log_vfs!("VFS EXEC: PROCESS REAP OK");

                    HELLO_COMPLETE.store(true, Ordering::Release);

                    crate::task::exit();
                } else {
                    crate::log_vfs!("VFS EXEC: reap FAILED");
                }
            }
        }
        crate::task::sleep(5);
    }
}

extern "C" fn open_test_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::log_vfs!("VFS EXEC: open_test reaper started");

    loop {
        let pid = REAP_OPEN_TEST_PID.load(Ordering::Relaxed);

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
                crate::log_vfs_raw!("VFS EXEC: open_test PID=");
                crate::log_vfs_raw!("{}", pid);
                crate::log_exec!(" exited, reaping...");

                let mut allocator = FrameAllocator::new();

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

                if ok && gone {
                    crate::log_vfs!("VFS EXEC: open_test PROCESS REAP OK");

                    OPEN_TEST_COMPLETE.store(true, Ordering::Release);

                    REAP_OPEN_TEST_PID.store(0, Ordering::Relaxed);

                    crate::task::exit();
                } else {
                    crate::log_vfs!("VFS EXEC: open_test reap FAILED");
                }
            }
        }

        crate::task::sleep(5);
    }
}

pub fn spawn_reaper(
    pid: u64,
    cpu_id: usize,
    func: extern "C" fn() -> !,
    reap_slot: &'static AtomicU64,
) -> bool {
    let tid = unsafe { (*crate::task::process_manager()).allocate_thread_id() };

    let thread = Thread::new(tid, cpu_id, 0, func);

    let added = unsafe { crate::task::SCHEDULERS[cpu_id].add_thread(thread, cpu_id) };

    if !added {
        crate::log_vfs!("VFS EXEC: reaper add_thread FAILED");
        return false;
    }

    unsafe {
        (*crate::task::process_manager()).add_thread(0, tid);
    }

    reap_slot.store(pid, Ordering::Relaxed);

    crate::log_vfs_raw!("VFS EXEC: reaper TID=");
    crate::log_vfs_raw!("{}", tid);
    crate::log_vfs!(" armed");

    true
}

pub fn test_hello_exec() -> bool {
    crate::log_vfs!("=== VFS EXEC HELLO TEST ===");

    HELLO_COMPLETE.store(false, Ordering::Release);
    REAP_PID.store(0, Ordering::Relaxed);

    let argv: &[&str] = &["hello"];
    let res = match exec_path(HELLO_PATH, argv, HELLO_CPU) {
        Ok(r) => r,
        Err(e) => {
            crate::log_vfs_raw!("VFS EXEC FAILED: ");
            crate::log_vfs!("{}", alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::log_vfs!("VFS EXEC: unexpected entry FAILED");
        return false;
    }
    crate::log_vfs!("VFS EXEC: ELF LOAD OK");

    if !spawn_reaper(res.process_id, HELLO_CPU, hello_reaper, &REAP_PID) {
        crate::log_vfs!("VFS EXEC: reaper spawn FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC: /bin/hello queued for Ring3");
    true
}

pub fn test_open_test_exec() -> bool {
    crate::log_vfs!("=== VFS EXEC OPEN TEST ===");

    OPEN_TEST_COMPLETE.store(false, Ordering::Release);
    REAP_OPEN_TEST_PID.store(0, Ordering::Relaxed);

    let cpu = if crate::arch::x86_64::smp::percpu::is_online(OPEN_TEST_CPU) {
        OPEN_TEST_CPU
    } else {
        HELLO_CPU
    };

    let argv: &[&str] = &["open_test"];

    let res = match exec_path(OPEN_TEST_PATH, argv, cpu) {
        Ok(r) => r,

        Err(e) => {
            crate::log_vfs_raw!("VFS EXEC OPEN TEST FAILED: ");
            crate::log_vfs!("{}", alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::log_vfs!("VFS EXEC OPEN TEST: unexpected entry FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC OPEN TEST: ELF LOAD OK");

    if !spawn_reaper(res.process_id, cpu, open_test_reaper, &REAP_OPEN_TEST_PID) {
        crate::log_vfs!("VFS EXEC OPEN TEST: reaper spawn FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC: /bin/open_test queued for Ring3");

    true
}

pub extern "C" fn init_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::log_vfs!("VFS EXEC: init reaper started");

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
                crate::log_vfs_raw!("VFS EXEC: init PID=");
                crate::log_vfs_raw!("{}", pid);
                crate::log_exec!(" exited, reaping...");

                let mut allocator = FrameAllocator::new();

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

                if ok && gone {
                    crate::log_vfs!("VFS EXEC: init PROCESS REAP OK");

                    INIT_COMPLETE.store(true, Ordering::Release);

                    crate::task::exit();
                } else {
                    crate::log_vfs!("VFS EXEC: init reap FAILED");
                }
            }
        }

        crate::task::sleep(5);
    }
}

pub extern "C" fn badptr_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::log_vfs!("VFS EXEC: badptr reaper started");

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
                crate::log_vfs_raw!("VFS EXEC: badptr PID=");
                crate::log_vfs_raw!("{}", pid);
                crate::log_exec!(" exited, reaping...");

                let mut allocator = FrameAllocator::new();

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

                if ok && gone {
                    crate::log_vfs!("VFS EXEC: badptr PROCESS REAP OK");

                    BADPTR_COMPLETE.store(true, Ordering::Release);

                    crate::task::exit();
                } else {
                    crate::log_vfs!("VFS EXEC: badptr reap FAILED");
                }
            }
        }

        crate::task::sleep(5);
    }
}

extern "C" fn read_test_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::log_vfs!("VFS EXEC: read_test reaper started");

    loop {
        let pid = REAP_READ_TEST_PID.load(Ordering::Relaxed);

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
                crate::log_vfs_raw!("VFS EXEC: read_test PID=");
                crate::log_vfs_raw!("{}", pid);
                crate::log_exec!(" exited, reaping...");

                let mut allocator = FrameAllocator::new();

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

                if ok && gone {
                    crate::log_vfs!("VFS EXEC: read_test PROCESS REAP OK");

                    READ_TEST_COMPLETE.store(true, Ordering::Release);

                    REAP_READ_TEST_PID.store(0, Ordering::Relaxed);

                    crate::task::exit();
                } else {
                    crate::log_vfs!("VFS EXEC: read_test reap FAILED");
                }
            }
        }

        crate::task::sleep(5);
    }
}

pub fn test_read_test_exec() -> bool {
    crate::log_vfs!("=== VFS EXEC READ TEST ===");

    READ_TEST_COMPLETE.store(false, Ordering::Release);
    REAP_READ_TEST_PID.store(0, Ordering::Relaxed);

    let cpu = if crate::arch::x86_64::smp::percpu::is_online(READ_TEST_CPU) {
        READ_TEST_CPU
    } else {
        HELLO_CPU
    };

    let argv: &[&str] = &["read_test"];

    let res = match exec_path(READ_TEST_PATH, argv, cpu) {
        Ok(r) => r,
        Err(e) => {
            crate::log_vfs_raw!("VFS EXEC READ TEST FAILED: ");
            crate::log_vfs!("{}", alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::log_vfs!("VFS EXEC READ TEST: unexpected entry FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC READ TEST: ELF LOAD OK");

    if !spawn_reaper(res.process_id, cpu, read_test_reaper, &REAP_READ_TEST_PID) {
        crate::log_vfs!("VFS EXEC READ TEST: reaper spawn FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC: /bin/read_test queued for Ring3");

    true
}

extern "C" fn close_test_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::log_vfs!("VFS EXEC: close_test reaper started");

    loop {
        let pid = REAP_CLOSE_TEST_PID.load(Ordering::Relaxed);

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
                crate::log_vfs_raw!("VFS EXEC: close_test PID=");
                crate::log_vfs_raw!("{}", pid);
                crate::log_exec!(" exited, reaping...");

                let mut allocator = FrameAllocator::new();

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

                if ok && gone {
                    crate::log_vfs!("VFS EXEC: close_test PROCESS REAP OK");

                    CLOSE_TEST_COMPLETE.store(true, Ordering::Release);

                    REAP_CLOSE_TEST_PID.store(0, Ordering::Relaxed);

                    crate::task::exit();
                } else {
                    crate::log_vfs!("VFS EXEC: close_test reap FAILED");
                }
            }
        }

        crate::task::sleep(5);
    }
}

pub fn test_close_test_exec() -> bool {
    crate::log_vfs!("=== VFS EXEC CLOSE TEST ===");

    CLOSE_TEST_COMPLETE.store(false, Ordering::Release);
    REAP_CLOSE_TEST_PID.store(0, Ordering::Relaxed);

    let cpu = if crate::arch::x86_64::smp::percpu::is_online(CLOSE_TEST_CPU) {
        CLOSE_TEST_CPU
    } else {
        HELLO_CPU
    };

    let argv: &[&str] = &["close_test"];

    let res = match exec_path(CLOSE_TEST_PATH, argv, cpu) {
        Ok(r) => r,

        Err(e) => {
            crate::log_vfs_raw!("VFS EXEC CLOSE TEST FAILED: ");
            crate::log_vfs!("{}", alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::log_vfs!("VFS EXEC CLOSE TEST: unexpected entry FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC CLOSE TEST: ELF LOAD OK");

    if !spawn_reaper(res.process_id, cpu, close_test_reaper, &REAP_CLOSE_TEST_PID) {
        crate::log_vfs!("VFS EXEC CLOSE TEST: reaper spawn FAILED");
        return false;
    }

    crate::log_vfs!("VFS EXEC: /bin/close_test queued for Ring3");

    true
}

extern "C" fn execve_test_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::log_vfs!("VFS EXEC: execve_test reaper started");

    loop {
        let pid = REAP_EXECVE_TEST_PID.load(Ordering::Relaxed);

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
                crate::log_vfs_raw!("VFS EXEC: execve_test PID=");
                crate::log_vfs_raw!("{}", pid);
                crate::log_exec!(" exited, reaping...");

                let mut allocator = FrameAllocator::new();

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

                if ok && gone {
                    crate::log_vfs!("VFS EXEC: execve_test PROCESS REAP OK");

                    EXECVE_TEST_COMPLETE.store(true, Ordering::Release);

                    crate::task::exit();
                } else {
                    crate::log_vfs!("VFS EXEC: execve_test reap FAILED");
                }
            }
        }

        crate::task::sleep(5);
    }
}

pub fn test_execve_exec() -> bool {
    crate::log_vfs!("=== VFS EXEC EXECVE TEST ===");

    EXECVE_TEST_COMPLETE.store(false, Ordering::Release);
    REAP_EXECVE_TEST_PID.store(0, Ordering::Relaxed);

    let argv: &[&str] = &["exec_test"];

    let res = match exec_path(EXECVE_TEST_PATH, argv, EXECVE_TEST_CPU) {
        Ok(r) => r,
        Err(e) => {
            crate::log_vfs_raw!("VFS EXEC EXECVE FAILED: ");
            crate::log_vfs!("{}", alloc::format!("{:?}", e).as_str(),);
            return false;
        }
    };

    crate::log_vfs_raw!("{}", "VFS EXEC: execve_test PROCESS CREATED PID=",);
    crate::log_vfs_raw!("{}", res.process_id);

    crate::log_vfs_raw!(" TID=");
    crate::log_vfs_raw!("{}", res.thread_id);

    crate::log_vfs!("");

    if !spawn_reaper(
        res.process_id,
        EXECVE_TEST_CPU,
        execve_test_reaper,
        &REAP_EXECVE_TEST_PID,
    ) {
        crate::log_vfs!("{}", "VFS EXEC: execve_test reaper spawn FAILED",);
        return false;
    }

    true
}

pub fn spawn_exec(
    path: &[u8],
    argv: &[&str],
    parent_pid: u64,
    cpu_id: usize,
) -> Result<ExecResult, VfsExecError> {
    crate::log_vfs_raw!("VFS SPAWN: open ");
    for &b in path {
        crate::log_vfs_raw!("{}", b as char);
    }
    crate::log_vfs!("");

    if !crate::arch::x86_64::smp::percpu::is_online(cpu_id) {
        crate::log_vfs!("VFS SPAWN: target CPU offline");
        return Err(VfsExecError::CpuOffline);
    }

    let (mut ide, vfs) = open_root_vfs()?;
    let elf = read_path(&vfs, &mut ide, path)?;

    crate::log_vfs_raw!("VFS SPAWN: ELF bytes=");
    crate::log_vfs_raw!("{}", elf.len() as u64);
    crate::log_vfs!(" magic OK");

    let mut allocator = FrameAllocator::new();
    let pm_ptr = crate::task::process_manager();

    let res = unsafe {
        ElfExecutor::exec_with_argv_on_cpu_parent(
            &elf,
            argv,
            &mut *pm_ptr,
            &mut allocator,
            cpu_id,
            Some(parent_pid),
        )
    }
    .map_err(VfsExecError::Exec)?;

    crate::log_vfs_raw!("VFS SPAWN: PROCESS CREATED PID=");
    crate::log_vfs_raw!("{}", res.process_id);
    crate::log_vfs_raw!(" PPID=");
    crate::log_vfs_raw!("{}", parent_pid);
    crate::log_vfs_raw!(" TID=");
    crate::log_vfs_raw!("{}", res.thread_id);
    crate::log_vfs_raw!(" entry=0x");
    crate::log_vfs_raw!("{:016X}", res.entry);
    crate::log_vfs!("");

    Ok(res)
}

pub fn test_hello_spawn() -> bool {
    crate::log_vfs!("=== VFS SPAWN HELLO TEST ===");

    let cpu = if crate::arch::x86_64::smp::percpu::is_online(HELLO_CPU) {
        HELLO_CPU
    } else {
        crate::log_vfs!("VFS SPAWN: target CPU offline");
        return false;
    };

    let parent_pid = {
        let mut allocator = FrameAllocator::new();
        let pm = crate::task::process_manager();

        unsafe {
            match (*pm).create_user(&mut allocator, None) {
                Some(pid) => pid,
                None => {
                    crate::log_vfs!("VFS SPAWN: parent creation FAILED");
                    return false;
                }
            }
        }
    };

    crate::log_vfs_raw!("VFS SPAWN: PARENT PID=");
    crate::log_vfs_raw!("{}", parent_pid);
    crate::log_vfs!("");

    let argv: &[&str] = &["hello"];

    let res = match spawn_exec(HELLO_PATH, argv, parent_pid, cpu) {
        Ok(r) => r,
        Err(e) => {
            crate::log_vfs_raw!("VFS SPAWN FAILED: ");
            crate::log_vfs!("{}", alloc::format!("{:?}", e).as_str());

            let mut allocator = FrameAllocator::new();
            unsafe {
                (*crate::task::process_manager()).remove_process(parent_pid, &mut allocator);
            }

            return false;
        }
    };

    let child_pid = res.process_id;

    crate::log_vfs_raw!("VFS SPAWN: CHILD PID=");
    crate::log_vfs_raw!("{}", child_pid);
    crate::log_vfs!("");

    let mut child_terminated = false;

    for i in 0..200 {
        let status = unsafe {
            let pm = crate::task::process_manager();

            match (*pm).get(child_pid) {
                Some(p) => {
                    let terminated = p.is_terminated();
                    let live_threads = (*pm).has_live_threads(child_pid);

                    crate::log_vfs_raw!("VFS SPAWN: child status iter=");
                    crate::log_vfs_raw!("{}", i as u64);
                    crate::log_vfs_raw!(" terminated=");
                    crate::log_vfs_raw!("{}", terminated as u64);
                    crate::log_vfs_raw!(" live_threads=");
                    crate::log_vfs_raw!("{}", live_threads as u64);
                    crate::log_vfs!("");

                    terminated && !live_threads
                }
                None => {
                    crate::log_vfs!("VFS SPAWN: child disappeared");
                    true
                }
            }
        };

        if status {
            child_terminated = true;
            break;
        }

        for _ in 0..50_000 {
            core::hint::spin_loop();
        }
    }

    if !child_terminated {
        crate::log_vfs!("VFS SPAWN: CHILD TERMINATION TIMEOUT");

        let mut allocator = FrameAllocator::new();

        unsafe {
            if (*crate::task::process_manager()).get(child_pid).is_some() {
                (*crate::task::process_manager()).remove_process(child_pid, &mut allocator);
            }

            if (*crate::task::process_manager()).get(parent_pid).is_some() {
                (*crate::task::process_manager()).remove_process(parent_pid, &mut allocator);
            }
        }

        return false;
    }

    crate::log_vfs_raw!("VFS SPAWN: CHILD TERMINATED PID=");
    crate::log_vfs_raw!("{}", child_pid);
    crate::log_vfs!("");

    let mut allocator = FrameAllocator::new();

    let reaped = unsafe { (*crate::task::process_manager()).reap(child_pid, &mut allocator) };

    if !reaped {
        crate::log_vfs!("VFS SPAWN: CHILD REAP FAILED");

        unsafe {
            if (*crate::task::process_manager()).get(parent_pid).is_some() {
                (*crate::task::process_manager()).remove_process(parent_pid, &mut allocator);
            }
        }

        return false;
    }

    let gone = unsafe { (*crate::task::process_manager()).get(child_pid).is_none() };

    if !gone {
        crate::log_vfs!("VFS SPAWN: CHILD STILL EXISTS");

        unsafe {
            if (*crate::task::process_manager()).get(parent_pid).is_some() {
                (*crate::task::process_manager()).remove_process(parent_pid, &mut allocator);
            }
        }

        return false;
    }

    crate::log_vfs!("VFS SPAWN: CHILD REAP OK");

    let parent_removed =
        unsafe { (*crate::task::process_manager()).remove_process(parent_pid, &mut allocator) };

    if !parent_removed {
        crate::log_vfs!("VFS SPAWN: PARENT CLEANUP FAILED");
        return false;
    }

    crate::log_vfs!("VFS SPAWN: PARENT CLEANUP OK");
    crate::log_vfs!("VFS SPAWN: TEST PASS");

    true
}
