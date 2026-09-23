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

/// Set by each CPU3 test reaper only after the user process was truly
/// reaped (ProcessManager::reap OK + process removed), just before the
/// reaper itself exits. Polled by the boot harness to serialize the
/// CPU3 VFS exec tests without touching scheduler/SMP/heap.
pub(crate) static BADPTR_COMPLETE: AtomicBool = AtomicBool::new(false);
pub(crate) static OPEN_TEST_COMPLETE: AtomicBool = AtomicBool::new(false);
pub(crate) static READ_TEST_COMPLETE: AtomicBool = AtomicBool::new(false);
pub(crate) static CLOSE_TEST_COMPLETE: AtomicBool = AtomicBool::new(false);

fn open_root_vfs() -> Result<(IdeDevice, Vfs), VfsExecError> {
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

    if data.len() < 4 || data[0] != 0x7F || data[1] != b'E' || data[2] != b'L' || data[3] != b'F' {
        return Err(VfsExecError::BadMagic);
    }

    Ok(data)
}

pub fn exec_path(path: &[u8], argv: &[&str], cpu_id: usize) -> Result<ExecResult, VfsExecError> {
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

extern "C" fn hello_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::serial::println("VFS EXEC: reaper started");

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
                crate::drivers::serial::print("VFS EXEC: PID=");
                crate::drivers::serial::print_u64(pid);
                crate::drivers::serial::println(" exited, reaping...");

                let mut allocator = FrameAllocator::new();
                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };
                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };
                if ok && gone {
                    crate::drivers::serial::println("VFS EXEC: PROCESS REAP OK");
                    crate::task::exit();
                } else {
                    crate::drivers::serial::println("VFS EXEC: reap FAILED");
                }
            }
        }
        crate::task::sleep(20);
    }
}

extern "C" fn open_test_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::serial::println("VFS EXEC: open_test reaper started");

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
                crate::drivers::serial::print("VFS EXEC: open_test PID=");
                crate::drivers::serial::print_u64(pid);
                crate::drivers::serial::println(" exited, reaping...");

                let mut allocator = FrameAllocator::new();

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

                if ok && gone {
                    crate::drivers::serial::println("VFS EXEC: open_test PROCESS REAP OK");

                    OPEN_TEST_COMPLETE.store(true, Ordering::Release);

                    crate::task::exit();
                } else {
                    crate::drivers::serial::println("VFS EXEC: open_test reap FAILED");
                }
            }
        }

        crate::task::sleep(20);
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
        crate::drivers::serial::println("VFS EXEC: reaper add_thread FAILED");
        return false;
    }

    unsafe {
        (*crate::task::process_manager()).add_thread(0, tid);
    }

    reap_slot.store(pid, Ordering::Relaxed);

    crate::drivers::serial::print("VFS EXEC: reaper TID=");
    crate::drivers::serial::print_u64(tid);
    crate::drivers::serial::println(" armed");

    true
}

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

    if !spawn_reaper(res.process_id, HELLO_CPU, hello_reaper, &REAP_PID) {
        crate::drivers::serial::println("VFS EXEC: reaper spawn FAILED");
        return false;
    }

    crate::drivers::serial::println("VFS EXEC: /bin/hello queued for Ring3");
    true
}

pub fn test_open_test_exec() -> bool {
    crate::drivers::serial::println("=== VFS EXEC OPEN TEST ===");

    let cpu = if crate::arch::x86_64::smp::percpu::is_online(OPEN_TEST_CPU) {
        OPEN_TEST_CPU
    } else {
        HELLO_CPU
    };

    let argv: &[&str] = &["open_test"];

    let res = match exec_path(OPEN_TEST_PATH, argv, cpu) {
        Ok(r) => r,

        Err(e) => {
            crate::drivers::serial::print("VFS EXEC OPEN TEST FAILED: ");
            crate::drivers::serial::println(alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::drivers::serial::println("VFS EXEC OPEN TEST: unexpected entry FAILED");
        return false;
    }

    crate::drivers::serial::println("VFS EXEC OPEN TEST: ELF LOAD OK");

    if !spawn_reaper(res.process_id, cpu, open_test_reaper, &REAP_OPEN_TEST_PID) {
        crate::drivers::serial::println("VFS EXEC OPEN TEST: reaper spawn FAILED");
        return false;
    }

    crate::drivers::serial::println("VFS EXEC: /bin/open_test queued for Ring3");

    true
}

pub extern "C" fn init_reaper() -> ! {
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

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

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

pub extern "C" fn badptr_reaper() -> ! {
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

                    BADPTR_COMPLETE.store(true, Ordering::Release);

                    crate::task::exit();
                } else {
                    crate::drivers::serial::println("VFS EXEC: badptr reap FAILED");
                }
            }
        }

        crate::task::sleep(20);
    }
}

extern "C" fn read_test_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::serial::println("VFS EXEC: read_test reaper started");

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
                crate::drivers::serial::print("VFS EXEC: read_test PID=");
                crate::drivers::serial::print_u64(pid);
                crate::drivers::serial::println(" exited, reaping...");

                let mut allocator = FrameAllocator::new();

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

                if ok && gone {
                    crate::drivers::serial::println("VFS EXEC: read_test PROCESS REAP OK");

                    READ_TEST_COMPLETE.store(true, Ordering::Release);

                    REAP_READ_TEST_PID.store(0, Ordering::Relaxed);

                    crate::task::exit();
                } else {
                    crate::drivers::serial::println("VFS EXEC: read_test reap FAILED");
                }
            }
        }

        crate::task::sleep(20);
    }
}

pub fn test_read_test_exec() -> bool {
    crate::drivers::serial::println("=== VFS EXEC READ TEST ===");

    let cpu = if crate::arch::x86_64::smp::percpu::is_online(READ_TEST_CPU) {
        READ_TEST_CPU
    } else {
        HELLO_CPU
    };

    let argv: &[&str] = &["read_test"];

    let res = match exec_path(READ_TEST_PATH, argv, cpu) {
        Ok(r) => r,
        Err(e) => {
            crate::drivers::serial::print("VFS EXEC READ TEST FAILED: ");
            crate::drivers::serial::println(alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::drivers::serial::println("VFS EXEC READ TEST: unexpected entry FAILED");
        return false;
    }

    crate::drivers::serial::println("VFS EXEC READ TEST: ELF LOAD OK");

    if !spawn_reaper(res.process_id, cpu, read_test_reaper, &REAP_READ_TEST_PID) {
        crate::drivers::serial::println("VFS EXEC READ TEST: reaper spawn FAILED");
        return false;
    }

    crate::drivers::serial::println("VFS EXEC: /bin/read_test queued for Ring3");

    true
}

extern "C" fn close_test_reaper() -> ! {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    crate::drivers::serial::println("VFS EXEC: close_test reaper started");

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
                crate::drivers::serial::print("VFS EXEC: close_test PID=");
                crate::drivers::serial::print_u64(pid);
                crate::drivers::serial::println(" exited, reaping...");

                let mut allocator = FrameAllocator::new();

                let ok = unsafe { (*crate::task::process_manager()).reap(pid, &mut allocator) };

                let gone = unsafe { (*crate::task::process_manager()).get(pid).is_none() };

                if ok && gone {
                    crate::drivers::serial::println("VFS EXEC: close_test PROCESS REAP OK");

                    CLOSE_TEST_COMPLETE.store(true, Ordering::Release);

                    REAP_CLOSE_TEST_PID.store(0, Ordering::Relaxed);

                    crate::task::exit();
                } else {
                    crate::drivers::serial::println("VFS EXEC: close_test reap FAILED");
                }
            }
        }

        crate::task::sleep(20);
    }
}

pub fn test_close_test_exec() -> bool {
    crate::drivers::serial::println("=== VFS EXEC CLOSE TEST ===");

    let cpu = if crate::arch::x86_64::smp::percpu::is_online(CLOSE_TEST_CPU) {
        CLOSE_TEST_CPU
    } else {
        HELLO_CPU
    };

    let argv: &[&str] = &["close_test"];

    let res = match exec_path(CLOSE_TEST_PATH, argv, cpu) {
        Ok(r) => r,

        Err(e) => {
            crate::drivers::serial::print("VFS EXEC CLOSE TEST FAILED: ");
            crate::drivers::serial::println(alloc::format!("{:?}", e).as_str());
            return false;
        }
    };

    if res.entry != 0x401000 {
        crate::drivers::serial::println("VFS EXEC CLOSE TEST: unexpected entry FAILED");
        return false;
    }

    crate::drivers::serial::println("VFS EXEC CLOSE TEST: ELF LOAD OK");

    if !spawn_reaper(res.process_id, cpu, close_test_reaper, &REAP_CLOSE_TEST_PID) {
        crate::drivers::serial::println("VFS EXEC CLOSE TEST: reaper spawn FAILED");
        return false;
    }

    crate::drivers::serial::println("VFS EXEC: /bin/close_test queued for Ring3");

    true
}
