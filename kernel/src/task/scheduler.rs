use super::thread::Thread;
use crate::arch::x86_64::context::{CpuContext, InterruptFrame, SyscallFrame};
use crate::drivers::serial;
use crate::task::thread::{self, ThreadID, ThreadState};

const MAX_THREADS: usize = 8;
const IDLE_SLOT: usize = MAX_THREADS - 1;

pub struct Scheduler {
    threads: [Option<Thread>; MAX_THREADS],
    current: usize,
    count: usize,
    switches: u64,
    ticks: u64,
    started: bool,
}

impl Scheduler {
    pub fn init_idle(&mut self, cpu_id: usize) {
        if self.threads[IDLE_SLOT].is_some() {
            return;
        }

        let idle = Thread::new(0, cpu_id, 0, idle_thread);

        self.threads[IDLE_SLOT] = Some(idle);
        self.threads[IDLE_SLOT]
            .as_mut()
            .unwrap()
            .set_state(ThreadState::Ready);

        self.count += 1;
    }

    pub const fn new() -> Self {
        Self {
            threads: [const { None }; MAX_THREADS],
            current: 0,
            count: 0,
            switches: 0,
            ticks: 0,
            started: false,
        }
    }

    pub fn wait_current(&mut self) {
        if let Some(thread) = self.current_mut() {
            thread.set_state(ThreadState::Waiting);
        }
    }

    pub fn wake_thread(&mut self, tid: ThreadID) -> bool {
        for thread in self.threads.iter_mut().flatten() {
            if thread.id() == tid {
                if thread.state() == ThreadState::Waiting {
                    thread.set_state(ThreadState::Ready);
                    return true;
                }
            }
        }

        false
    }

    pub fn wait_for_child(&mut self, process_id: crate::task::process::ProcessID) {
        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_waiting_for_process(Some(process_id));
            thread.set_state(ThreadState::Waiting);
        }
    }

    pub fn wait_for_any_child(&mut self) {
        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_waiting_for_process(None);
            thread.set_state(ThreadState::Waiting);
        }
    }

    pub fn wait_current_if(&mut self, condition: bool) -> bool {
        if !condition {
            return false;
        }

        if let Some(thread) = self.current_mut() {
            thread.set_state(ThreadState::Waiting);
            return true;
        }

        false
    }

    pub fn add_thread(&mut self, thread: Thread, cpu_id: usize) -> bool {
        if thread.cpu_id() != cpu_id {
            return false;
        }

        if self.count >= MAX_THREADS {
            return false;
        }

        for slot in self.threads.iter_mut() {
            if slot.is_none() {
                *slot = Some(thread);
                self.count += 1;
                return true;
            }
        }

        false
    }

    pub fn current(&self) -> Option<&Thread> {
        if self.count == 0 {
            return None;
        }

        self.threads[self.current].as_ref()
    }

    pub fn current_mut(&mut self) -> Option<&mut Thread> {
        if self.count == 0 {
            return None;
        }

        self.threads[self.current].as_mut()
    }

    pub fn current_id(&self) -> Option<ThreadID> {
        self.current().map(|thread| thread.id())
    }

    pub fn next(&self) -> Option<usize> {
        if self.count == 0 {
            return None;
        }

        let current = self.current;
        for offset in 1..MAX_THREADS {
            let index = (current + offset) % MAX_THREADS;

            if index == IDLE_SLOT {
                continue;
            }

            if let Some(thread) = &self.threads[index] {
                if thread.state() == ThreadState::Ready {
                    return Some(index);
                }
            }
        }

        if let Some(thread) = &self.threads[IDLE_SLOT] {
            if thread.state() == ThreadState::Ready || thread.state() == ThreadState::Running {
                return Some(IDLE_SLOT);
            }
        }

        None
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn thread(&self, index: usize) -> Option<&Thread> {
        self.threads.get(index).and_then(|thread| thread.as_ref())
    }

    pub fn thread_mut(&mut self, index: usize) -> Option<&mut Thread> {
        self.threads
            .get_mut(index)
            .and_then(|thread| thread.as_mut())
    }

    pub fn current_index(&self) -> usize {
        self.current
    }

    #[inline]
    fn user_entry_trampoline() -> u64 {
        crate::arch::x86_64::context::start_user_thread_context as usize as u64
    }

    fn validate_fresh_user(&self, index: usize) -> Option<&'static str> {
        let thread = self.threads[index].as_ref()?;

        if thread.is_kernel() {
            return Some("not a user thread");
        }

        let frame = unsafe { &*thread.user_frame_ptr() };

        if frame.cs != crate::arch::x86_64::gdt::GDT_USER_CODE as u64 {
            return Some("bad user CS");
        }

        if frame.ss != crate::arch::x86_64::gdt::GDT_USER_DATA as u64 {
            return Some("bad user SS");
        }

        if frame.rflags & 0x2 == 0 {
            return Some("bad RFLAGS (bit1 clear)");
        }

        if frame.rflags & (1 << 9) == 0 {
            return Some("bad RFLAGS (IF clear)");
        }

        if thread.interrupt_rsp() % 16 != 0 {
            return Some("user frame misaligned");
        }

        let rip = frame.rip;
        let rsp = frame.rsp;

        const USER_MAX: u64 = 0x0000_8000_0000_0000;

        if rip >= USER_MAX || rsp >= USER_MAX {
            return Some("RIP/RSP outside user half");
        }

        if rsp & 0x7 != 0 {
            return Some("user RSP unaligned");
        }

        let pid = thread.process_id();

        unsafe {
            let manager = crate::task::process_manager();

            match (*manager).get(pid) {
                Some(process) => match process.address_space() {
                    Some(space) => {
                        match space.probe_page(rip) {
                            Some((user, _, exec)) if user && exec => {}
                            _ => return Some("user RIP not executable-mapped"),
                        }

                        match space.probe_page(rsp) {
                            Some((user, writable, _)) if user && writable => {}
                            _ => return Some("user RSP not mapped"),
                        }
                    }
                    None => return Some("process has no address space"),
                },
                None => return Some("process not found"),
            }
        }

        None
    }

    fn log_user_entry(&self, index: usize) {
        let thread = match self.threads[index].as_ref() {
            Some(thread) => thread,
            None => return,
        };

        let frame = unsafe { &*thread.user_frame_ptr() };

        let cr3 = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

        let rsp0 = {
            let cpu = crate::arch::x86_64::smp::percpu::current();

            unsafe { (*cpu.tss.get()).kernel_stack() }
        };

        crate::log_ring3_raw!("ENTERING RING3: slot=");
        crate::log_sched_raw!("{}", index as u64);
        crate::log_sched_raw!(" tid=");
        crate::log_sched_raw!("{}", thread.id());
        crate::log_sched_raw!(" RIP=0x");
        crate::log_sched_raw!("{:016X}", frame.rip);
        crate::log_sched_raw!(" CS=0x");
        crate::log_sched_raw!("{:016X}", frame.cs);
        crate::log_sched_raw!(" RSP=0x");
        crate::log_sched_raw!("{:016X}", frame.rsp);
        crate::log_sched_raw!(" SS=0x");
        crate::log_sched_raw!("{:016X}", frame.ss);
        crate::log_sched!("");
        crate::log_cr3_raw!("RING3 CTX: CR3=0x");
        crate::log_sched_raw!("{:016X}", cr3);
        crate::log_tss_raw!(" TSS.RSP0=0x");
        crate::log_sched_raw!("{:016X}", rsp0);
        crate::log_sched!("");

        Self::debug_tss_delivery(rsp0);
    }

    fn debug_tss_delivery(rsp0: u64) {
        let tr_selector: u16;
        let gdtr_base: u64;
        let gdtr_limit: u16;

        unsafe {
            core::arch::asm!("str {:x}", out(reg) tr_selector, options(nomem, nostack, preserves_flags));
            let mut gdtr = [0u8; 10];
            core::arch::asm!("sgdt [{}]", in(reg) gdtr.as_mut_ptr(), options(nostack, preserves_flags));
            gdtr_limit = u16::from_le_bytes([gdtr[0], gdtr[1]]);
            gdtr_base = u64::from_le_bytes([
                gdtr[2], gdtr[3], gdtr[4], gdtr[5], gdtr[6], gdtr[7], gdtr[8], gdtr[9],
            ]);
        }

        crate::log_tss_raw!("TSS AUDIT: TR=0x");
        crate::log_sched_raw!("{:016X}", tr_selector as u64);
        crate::log_sched_raw!(" GDTR base=0x");
        crate::log_sched_raw!("{:016X}", gdtr_base);
        crate::log_sched_raw!(" limit=");
        crate::log_sched_raw!("{}", gdtr_limit as u64);
        crate::log_sched!("");

        let index = (tr_selector >> 3) as u64;
        let desc_addr = gdtr_base + index * 8;

        let (low, high) = unsafe {
            (
                core::ptr::read_volatile(desc_addr as *const u64),
                core::ptr::read_volatile((desc_addr + 8) as *const u64),
            )
        };

        let base = ((low >> 16) & 0xFFFF)
            | (((low >> 32) & 0xFF) << 16)
            | (((low >> 56) & 0xFF) << 24)
            | (high << 32);
        let limit = (low & 0xFFFF) | (((low >> 48) & 0xF) << 16);
        let present = (low >> 47) & 1;
        let desc_type = (low >> 40) & 0xF;

        crate::log_tss_raw!("TSS AUDIT: desc base=0x");
        crate::log_sched_raw!("{:016X}", base);
        crate::log_sched_raw!(" limit=");
        crate::log_sched_raw!("{}", limit);
        crate::log_sched_raw!(" P=");
        crate::log_sched_raw!("{}", present);
        crate::log_sched_raw!(" type=0x");
        crate::log_sched_raw!("{:016X}", desc_type);
        crate::log_sched!("");

        let cpu_rsp0 = unsafe { core::ptr::read_volatile((base + 4) as *const u64) };

        crate::log_tss_raw!("TSS AUDIT: cpu-visible RSP0=0x");
        crate::log_sched_raw!("{:016X}", cpu_rsp0);
        crate::log_tss_raw!(" percpu RSP0=0x");
        crate::log_sched_raw!("{:016X}", rsp0);
        crate::log_sched!("");

        {
            let cpu = crate::arch::x86_64::smp::percpu::current();
            let true_tss = unsafe { cpu.tss.get() as u64 };
            let percpu_base = cpu as *const _ as u64;

            crate::log_tss_raw!("TSS AUDIT: percpu=0x");
            crate::log_sched_raw!("{:016X}", percpu_base);
            crate::log_tss_raw!(" tss=0x");
            crate::log_sched_raw!("{:016X}", true_tss);
            crate::log_sched_raw!(" desc=0x");
            crate::log_sched_raw!("{:016X}", base);

            if base == true_tss && cpu_rsp0 == rsp0 {
                crate::log_sched!(" MATCH");
            } else {
                crate::log_sched!(" MISMATCH");
            }
        }

        let cr3 = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

        let hhdm = |phys: u64| crate::boot::limine::physical_to_virtual(phys);

        let mut table_phys = cr3;
        let mut ok = true;

        for _level in 0..4 {
            let shift = 39 - _level * 9;
            let idx = ((rsp0 >> shift) & 0x1FF) as usize;

            let va = match hhdm(table_phys) {
                Some(va) => va,
                None => {
                    ok = false;
                    break;
                }
            };

            let entry = unsafe { core::ptr::read_volatile((va + idx as u64 * 8) as *const u64) };

            if entry & 1 == 0 || (entry & (1 << 7) != 0) {
                ok = false;
                break;
            }

            table_phys = entry & 0x000F_FFFF_FFFF_F000;
        }

        if ok {
            crate::log_tss!("TSS AUDIT: RSP0 page walk OK");
        } else {
            crate::log_tss!("TSS AUDIT: RSP0 page walk FAILED");
        }
    }

    fn target_pml4(&self, index: usize) -> Option<u64> {
        let thread = self.threads[index].as_ref()?;

        if thread.is_kernel() {
            return crate::memory::virtualm::try_kernel_pml4().map(|frame| frame.address);
        }

        let pid = thread.process_id();

        unsafe {
            let manager = crate::task::process_manager();

            (*manager).get(pid).and_then(|process| process.pml4_phys())
        }
    }

    fn switch_address_space_for(&self, next: usize) {
        let target = match self.target_pml4(next) {
            Some(pml4) => pml4,
            None => return,
        };

        let current = crate::arch::x86_64::paging::read_cr3() & !0xFFF;

        if target != current {
            crate::log_cr3_raw!("CR3 SWITCH: 0x");
            crate::log_sched_raw!("{}", current);
            crate::log_sched_raw!(" -> 0x");
            crate::log_sched_raw!("{}", target);
            crate::log_sched!("");

            unsafe {
                crate::arch::x86_64::paging::load_cr3(target);
            }
        }

        let is_user = self.threads[next]
            .as_ref()
            .map(|t| t.is_user())
            .unwrap_or(false);

        if is_user {
            let stack_top = self.threads[next]
                .as_ref()
                .map(|t| t.stack_top())
                .unwrap_or(0);

            if stack_top != 0 {
                let cpu = crate::arch::x86_64::smp::percpu::current();

                unsafe {
                    (*cpu.tss.get()).set_kernel_stack(stack_top);
                }

                crate::log_tss_raw!("TSS.RSP0 configured: CPU=");
                crate::log_sched_raw!(
                    "{}",
                    crate::arch::x86_64::smp::percpu::current_cpu_id() as u64,
                );
                crate::log_tss_raw!(" RSP0=0x");
                crate::log_sched_raw!("{:016X}", stack_top);
                crate::log_sched!("");
            }
        }
    }

    pub fn start_first(&mut self) -> ! {
        let first = (0..MAX_THREADS).find(|&index| self.threads[index].is_some());

        let first = match first {
            Some(index) => index,
            None => loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            },
        };

        self.current = first;

        if let Some(thread) = self.threads[first].as_mut() {
            thread.set_state(ThreadState::Running);
            thread.mark_started();
        }
        let is_user = self.threads[first]
            .as_ref()
            .map(|thread| thread.is_user())
            .unwrap_or(false);

        if is_user {
            unsafe {
                core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
            }
        }

        crate::log_vfs_raw!("Scheduler: starting thread slot ");
        crate::log_sched_raw!("{}", first as u64);
        crate::log_sched!("");

        self.switch_address_space_for(first);

        self.started = true;

        if is_user {
            if let Some(reason) = self.validate_fresh_user(first) {
                crate::log_ring3_raw!("RING3 REFUSED (start_first): ");
                crate::log_sched!("{}", reason);

                if let Some(thread) = self.threads[first].as_mut() {
                    thread.set_state(ThreadState::Terminated);
                }

                unsafe {
                    core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
                }

                loop {
                    unsafe {
                        core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
                    }
                }
            }

            self.log_user_entry(first);

            let frame = self.threads[first]
                .as_ref()
                .map(|thread| thread.user_frame_ptr())
                .unwrap();

            unsafe {
                crate::arch::x86_64::context::start_user_thread_context(frame);
            }
        }

        let (stack_top, entry) = match self.threads[first].as_ref() {
            Some(thread) => (thread.stack_top(), thread.entry()),
            None => loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            },
        };

        unsafe {
            crate::arch::x86_64::context::start_first_thread(stack_top, entry as *const ());
        }
    }

    fn reschedule_current(&mut self, frame: *mut InterruptFrame) -> (u64, u64) {
        if self.count == 0 || !self.started {
            return (frame as u64, 0);
        }

        self.reap_terminated();

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_interrupt_rsp(frame as u64);
        }

        let next = match self.next() {
            Some(index) => index,
            None => return (frame as u64, 0),
        };

        if next == self.current {
            return (frame as u64, 0);
        }

        let prev = self.current;

        if let Some(thread) = self.threads[prev].as_mut() {
            if thread.state() == ThreadState::Running {
                thread.set_state(ThreadState::Ready);
            }
        }

        if let Some(thread) = self.threads[next].as_mut() {
            thread.set_state(ThreadState::Running);
        }

        self.current = next;
        self.switches = self.switches.wrapping_add(1);

        let fresh = match self.threads[next].as_mut() {
            Some(thread) if thread.is_fresh() => {
                thread.mark_started();
                true
            }
            _ => false,
        };

        self.switch_address_space_for(next);

        if fresh {
            let is_user = self.threads[next]
                .as_ref()
                .map(|thread| thread.is_user())
                .unwrap_or(false);

            if is_user {
                if let Some(reason) = self.validate_fresh_user(next) {
                    crate::log_ring3_raw!("RING3 REFUSED: ");
                    crate::log_sched!("{}", reason);

                    if let Some(thread) = self.threads[next].as_mut() {
                        thread.set_state(ThreadState::Terminated);
                    }

                    return (frame as u64, 0);
                }

                self.log_user_entry(next);

                let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

                return (rsp, Self::user_entry_trampoline());
            }

            let thread = self.threads[next].as_ref().unwrap();

            return (thread.stack_top(), thread.entry());
        }

        let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

        (rsp, 0)
    }

    pub fn reschedule(&mut self, frame: *mut InterruptFrame) -> (u64, u64) {
        self.reschedule_current(frame)
    }

    pub fn tick(&mut self, frame: *mut InterruptFrame) -> (u64, u64) {
        if self.count == 0 {
            return (frame as u64, 0);
        }
        if !self.started {
            return (frame as u64, 0);
        }

        self.reap_terminated();

        if crate::arch::x86_64::smp::percpu::current_cpu_id() != 0 {
            let cpu = crate::arch::x86_64::smp::percpu::current_cpu_id();
            self.ticks = self.ticks.wrapping_add(1);

            if self.ticks % 100 == 0 {
                crate::log_sched_raw!("AP Scheduler tick: CPU=");
                crate::log_sched_raw!("{}", cpu as u64);
                crate::log_sched_raw!(" ticks=");
                crate::log_sched_raw!("{}", self.ticks);
                crate::log_sched!("");
            }

            if let Some(thread) = self.threads[self.current].as_mut() {
                thread.set_interrupt_rsp(frame as u64);
            }

            let now = crate::arch::x86_64::lapic::ticks();

            for thread in self.threads.iter_mut() {
                if let Some(thread) = thread.as_mut() {
                    if thread.state() == ThreadState::Sleeping && thread.wake_at() <= now {
                        thread.set_state(ThreadState::Ready);
                        crate::log_smp_raw!("CPU=");
                        crate::log_sched_raw!("{}", cpu as u64);
                        crate::log_sched_raw!(" TID=");
                        crate::log_sched_raw!("{}", thread.id());
                        crate::log_sched!(" WAKE");
                    }
                }
            }

            let next = match self.next() {
                Some(index) => index,
                None => {
                    return (frame as u64, 0);
                }
            };

            if next == self.current {
                return (frame as u64, 0);
            }

            let prev = self.current;
            let prev_tid = self.threads[prev].as_ref().map(|t| t.id()).unwrap_or(99);
            let next_tid = self.threads[next].as_ref().map(|t| t.id()).unwrap_or(99);

            if let Some(thread) = self.threads[self.current].as_mut() {
                if thread.state() == ThreadState::Running {
                    thread.set_state(ThreadState::Ready);
                }
            }

            if let Some(thread) = self.threads[next].as_mut() {
                thread.set_state(ThreadState::Running);
            }

            self.current = next;
            self.switches = self.switches.wrapping_add(1);

            crate::log_smp_raw!("CPU=");
            crate::log_sched_raw!("{}", cpu as u64);
            crate::log_sched_raw!(" current=");
            crate::log_sched_raw!("{}", prev as u64);
            crate::log_sched_raw!(" -> next=");
            crate::log_sched_raw!("{}", next as u64);
            crate::log_sched!("");
            if prev == IDLE_SLOT && next != IDLE_SLOT {
                crate::log_smp_raw!("CPU=");
                crate::log_sched_raw!("{}", cpu as u64);
                crate::log_sched_raw!(" IDLE -> TID=");
                crate::log_sched_raw!("{}", next_tid);
                crate::log_sched!("");
            } else if prev != IDLE_SLOT && next == IDLE_SLOT {
                crate::log_smp_raw!("CPU=");
                crate::log_sched_raw!("{}", cpu as u64);
                crate::log_sched_raw!(" TID=");
                crate::log_sched_raw!("{}", prev_tid);
                crate::log_sched!(" -> IDLE");
            }

            let fresh = match self.threads[next].as_mut() {
                Some(thread) if thread.is_fresh() => {
                    thread.mark_started();
                    true
                }
                _ => false,
            };

            self.switch_address_space_for(next);

            if fresh {
                let is_user = self.threads[next]
                    .as_ref()
                    .map(|thread| thread.is_user())
                    .unwrap_or(false);

                if is_user {
                    if let Some(reason) = self.validate_fresh_user(next) {
                        crate::log_ring3_raw!("RING3 REFUSED: ");
                        crate::log_sched!("{}", reason);

                        if let Some(thread) = self.threads[next].as_mut() {
                            thread.set_state(ThreadState::Terminated);
                        }

                        return (frame as u64, 0);
                    }

                    self.log_user_entry(next);

                    let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

                    return (rsp, Self::user_entry_trampoline());
                }

                let thread = self.threads[next].as_ref().unwrap();

                let stack = thread.stack_top();
                let entry = thread.entry();

                return (stack, entry);
            } else {
                let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

                return (rsp, 0);
            }
        }

        let now = crate::arch::x86_64::lapic::ticks();
        crate::log_sched_raw!("TIME=");
        crate::log_sched_raw!("{}", now);
        crate::log_sched!("");

        for thread in self.threads.iter_mut() {
            if let Some(thread) = thread.as_mut() {
                if thread.state() == ThreadState::Sleeping {
                    crate::log_sleep_raw!("SLEEP CHECK thread=");
                    crate::log_sched_raw!("{}", thread.id());

                    crate::log_sched_raw!(" wake=");
                    crate::log_sched_raw!("{}", thread.wake_at());

                    crate::log_sched_raw!(" now=");
                    crate::log_sched_raw!("{}", now);

                    if thread.wake_at() <= now {
                        thread.set_state(ThreadState::Ready);
                        crate::log_sched!(" -> WAKE");
                    } else {
                        crate::log_sched!(" -> WAIT");
                    }
                }
            }
        }

        crate::log_sched_raw!("SCHED tick CPU=");
        crate::log_sched_raw!(
            "{}",
            crate::arch::x86_64::smp::percpu::current_cpu_id() as u64
        );
        crate::log_sched_raw!(" current=");
        crate::log_sched_raw!("{}", self.current as u64);
        crate::log_sched_raw!(" next=");

        match self.next() {
            Some(index) => crate::log_sched_raw!("{}", index as u64),
            None => crate::log_sched_raw!("{}", 99),
        }

        crate::log_sched!("");

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_interrupt_rsp(frame as u64);
        }

        let next = match self.next() {
            Some(index) => index,
            None => {
                return (frame as u64, 0);
            }
        };

        if next == self.current {
            return (frame as u64, 0);
        }

        if let Some(thread) = self.threads[self.current].as_mut() {
            if thread.state() == ThreadState::Running {
                thread.set_state(ThreadState::Ready);
            }
        }

        if let Some(thread) = self.threads[next].as_mut() {
            thread.set_state(ThreadState::Running);
        }

        self.current = next;
        self.switches = self.switches.wrapping_add(1);

        if self.switches % 200 == 0 {
            crate::log_sched_raw!("Scheduler: ");
            crate::log_sched_raw!("{}", self.switches);
            crate::log_sched_raw!(" switches, now on slot ");
            crate::log_sched_raw!("{}", next as u64);
            crate::log_sched!("");
        }

        let fresh = match self.threads[next].as_mut() {
            Some(thread) if thread.is_fresh() => {
                thread.mark_started();
                true
            }
            _ => false,
        };

        self.switch_address_space_for(next);

        if fresh {
            let is_user = self.threads[next]
                .as_ref()
                .map(|thread| thread.is_user())
                .unwrap_or(false);

            if is_user {
                if let Some(reason) = self.validate_fresh_user(next) {
                    crate::log_ring3_raw!("RING3 REFUSED: ");
                    crate::log_sched!("{}", reason);

                    if let Some(thread) = self.threads[next].as_mut() {
                        thread.set_state(ThreadState::Terminated);
                    }

                    return (frame as u64, 0);
                }

                self.log_user_entry(next);

                let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

                crate::log_ring3_raw!("FRESH USER SWITCH: slot=");
                crate::log_sched_raw!("{}", next as u64);
                crate::log_vfs_raw!(" thread=");
                crate::log_sched_raw!(
                    "{}",
                    self.threads[next].as_ref().map(|t| t.id()).unwrap_or(0),
                );
                crate::log_vfs!(" via start_user_thread_context");

                return (rsp, Self::user_entry_trampoline());
            }

            let thread = self.threads[next].as_ref().unwrap();

            let stack = thread.stack_top();
            let entry = thread.entry();

            crate::log_sched_raw!("FRESH SWITCH: slot=");
            crate::log_sched_raw!("{}", next as u64);

            crate::log_vfs_raw!(" thread=");
            crate::log_sched_raw!("{}", thread.id());

            crate::log_sched_raw!(" entry=");
            crate::log_sched_raw!("{}", entry);

            crate::log_sched_raw!(" stack=");
            crate::log_sched_raw!("{}", stack);

            crate::log_sched!("");

            (stack, entry)
        } else {
            let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

            (rsp, 0)
        }
    }

    pub fn syscall_yield(&mut self, frame: *mut SyscallFrame) -> (u64, u64) {
        if self.count == 0 || !self.started {
            return (frame as u64, 0);
        }

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_interrupt_rsp(frame as u64);
        }

        let next = match self.next() {
            Some(index) => index,
            None => return (frame as u64, 0),
        };

        if next == self.current {
            return (frame as u64, 0);
        }

        if let Some(thread) = self.threads[self.current].as_mut() {
            if thread.state() == ThreadState::Running {
                thread.set_state(ThreadState::Ready);
            }
        }

        if let Some(thread) = self.threads[next].as_mut() {
            thread.set_state(ThreadState::Running);
        }

        self.current = next;
        self.switches = self.switches.wrapping_add(1);

        let fresh = match self.threads[next].as_mut() {
            Some(thread) if thread.is_fresh() => {
                thread.mark_started();
                true
            }
            _ => false,
        };

        self.switch_address_space_for(next);

        if fresh {
            let is_user = self.threads[next]
                .as_ref()
                .map(|thread| thread.is_user())
                .unwrap_or(false);

            if is_user {
                if let Some(reason) = self.validate_fresh_user(next) {
                    crate::log_syscall_raw!("RING3 REFUSED (syscall yield): ");
                    crate::log_sched!("{}", reason);

                    if let Some(thread) = self.threads[next].as_mut() {
                        thread.set_state(ThreadState::Terminated);
                    }

                    return (frame as u64, 0);
                }

                self.log_user_entry(next);

                let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

                return (rsp, Self::user_entry_trampoline());
            }

            let thread = self.threads[next].as_ref().unwrap();

            let stack = thread.stack_top();
            let entry = thread.entry();

            return (stack, entry);
        }

        let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

        (rsp, 0)
    }

    pub fn syscall_exit(&mut self, _frame: *mut SyscallFrame) -> (u64, u64) {
        if self.count == 0 || !self.started {
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }

        self.exit_current();

        let next = match self.next() {
            Some(index) => index,
            None => {
                if let Some(idle) = self.threads[IDLE_SLOT].as_ref().map(|_| IDLE_SLOT) {
                    idle
                } else {
                    loop {
                        unsafe {
                            core::arch::asm!("hlt");
                        }
                    }
                }
            }
        };

        if let Some(thread) = self.threads[next].as_mut() {
            thread.set_state(ThreadState::Running);
        }

        self.current = next;
        self.switches = self.switches.wrapping_add(1);

        let fresh = match self.threads[next].as_mut() {
            Some(thread) if thread.is_fresh() => {
                thread.mark_started();
                true
            }
            _ => false,
        };

        self.switch_address_space_for(next);

        if fresh {
            let is_user = self.threads[next]
                .as_ref()
                .map(|thread| thread.is_user())
                .unwrap_or(false);

            if is_user {
                if let Some(reason) = self.validate_fresh_user(next) {
                    crate::log_syscall_raw!("RING3 REFUSED (syscall exit): ");
                    crate::log_sched!("{}", reason);

                    if let Some(thread) = self.threads[next].as_mut() {
                        thread.set_state(ThreadState::Terminated);
                    }

                    let fallback = match self.next() {
                        Some(index) => index,
                        None => loop {
                            unsafe {
                                core::arch::asm!("hlt");
                            }
                        },
                    };

                    if let Some(thread) = self.threads[fallback].as_mut() {
                        thread.set_state(ThreadState::Running);
                    }

                    self.current = fallback;
                    self.switch_address_space_for(fallback);

                    let rsp = self.threads[fallback].as_ref().unwrap().interrupt_rsp();

                    return (rsp, 0);
                }

                self.log_user_entry(next);

                let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

                return (rsp, Self::user_entry_trampoline());
            }

            let thread = self.threads[next].as_ref().unwrap();

            let stack = thread.stack_top();
            let entry = thread.entry();

            return (stack, entry);
        }

        let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

        (rsp, 0)
    }

    pub fn sleep_current(&mut self, ticks: u64) {
        let now = crate::arch::x86_64::lapic::ticks();
        let cpu = crate::arch::x86_64::smp::percpu::current_cpu_id();

        crate::log_sched_raw!("TIME=");
        crate::log_sched_raw!("{}", now);
        crate::log_sched!("");

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_wake_at(now + ticks);
            thread.set_state(ThreadState::Sleeping);

            crate::log_sleep_raw!("SLEEP thread=");
            crate::log_sched_raw!("{}", thread.id());
            crate::log_sched_raw!(" wake_at=");
            crate::log_sched_raw!("{}", now + ticks);
            crate::log_sched!("");

            crate::log_smp_raw!("CPU=");
            crate::log_sched_raw!("{}", cpu as u64);
            crate::log_sched_raw!(" TID=");
            crate::log_sched_raw!("{}", thread.id());
            crate::log_sleep!(" SLEEP");
        }
    }

    pub fn exit_current(&mut self) {
        let (exiting_id, pid) = match self.threads[self.current].as_ref() {
            Some(thread) => (thread.id(), thread.process_id()),
            None => return,
        };

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_state(ThreadState::Terminated);

            crate::log_vfs_raw!("EXIT thread=");
            crate::log_sched_raw!("{}", exiting_id);
            crate::log_sched!("");
        }

        let mut process_terminated = false;

        unsafe {
            let pm = crate::task::process_manager();

            process_terminated = (*pm).notify_thread_terminated(pid, exiting_id);
        }

        if process_terminated {
            let parent_of_terminated = unsafe {
                let pm = crate::task::process_manager();
                (*pm).parent_of(pid)
            };
            let current_cpu = crate::arch::x86_64::smp::percpu::current_cpu_id();
            for cpu_id in 0..crate::arch::x86_64::smp::percpu::MAX_CPUS {
                let sched_ptr = unsafe { core::ptr::addr_of_mut!(crate::task::SCHEDULERS[cpu_id]) };
                let sched = unsafe { &mut *sched_ptr };
                for index in 0..MAX_THREADS {
                    if cpu_id == current_cpu && index == self.current {
                        continue;
                    }
                    if let Some(thread) = sched.threads[index].as_mut() {
                        let specific = thread.state() == ThreadState::Waiting
                            && thread.waiting_for_process() == Some(pid);
                        let generic = thread.state() == ThreadState::Waiting
                            && thread.waiting_for_process().is_none()
                            && parent_of_terminated == Some(thread.process_id());
                        if specific || generic {
                            let frame_ptr = thread.interrupt_rsp() as *mut SyscallFrame;
                            if !frame_ptr.is_null() {
                                unsafe {
                                    (*frame_ptr).rax = pid;
                                }
                            }
                            thread.set_waiting_for_process(None);
                            thread.set_state(ThreadState::Ready);
                            crate::log_vfs_raw!("WAIT: waking parent thread=");
                            crate::log_sched_raw!("{}", thread.id());
                            crate::log_sched_raw!(" child PID=");
                            crate::log_sched_raw!("{}", pid);
                            crate::log_sched!("");
                            crate::log_sched_raw!("WAIT: waking parent PID=");
                            crate::log_sched_raw!("{}", thread.process_id());
                            crate::log_sched_raw!(" TID=");
                            crate::log_sched_raw!("{}", thread.id());
                            crate::log_sched_raw!(" child PID=");
                            crate::log_sched_raw!("{}", pid);
                            crate::log_sched!("");
                            crate::log_sched_raw!("WAIT: parent resumed RAX=");
                            crate::log_sched_raw!("{}", pid);
                            crate::log_sched!("");
                            crate::log_sched_raw!("WAIT: child PID=");
                            crate::log_sched_raw!("{}", pid);
                            crate::log_sched_raw!(" terminated");
                            crate::log_sched!("");
                        }
                    }
                }
            }
        }

        {
            let current_cpu = crate::arch::x86_64::smp::percpu::current_cpu_id();
            for cpu_id in 0..crate::arch::x86_64::smp::percpu::MAX_CPUS {
                let sched_ptr = unsafe { core::ptr::addr_of_mut!(crate::task::SCHEDULERS[cpu_id]) };
                let sched = unsafe { &mut *sched_ptr };
                for index in 0..MAX_THREADS {
                    if cpu_id == current_cpu && index == self.current {
                        continue;
                    }
                    if let Some(thread) = sched.threads[index].as_mut() {
                        if thread.state() == ThreadState::Waiting
                            && thread.waiting_for() == Some(exiting_id)
                        {
                            thread.set_waiting_for(None);
                            thread.set_state(ThreadState::Ready);
                            crate::log_vfs_raw!("JOIN: waking thread=");
                            crate::log_sched_raw!("{}", thread.id());
                            crate::log_sched!("");
                        }
                    }
                }
            }
        }
    }

    pub fn kill(&mut self, id: ThreadID) -> bool {
        let mut pid_opt = None;
        for index in 0..MAX_THREADS {
            if let Some(thread) = self.threads[index].as_mut() {
                if thread.id() == id {
                    thread.set_state(ThreadState::Terminated);
                    pid_opt = Some(thread.process_id());

                    crate::log_vfs_raw!("KILL thread=");
                    crate::log_sched_raw!("{}", id);
                    crate::log_sched!("");

                    break;
                }
            }
        }
        if let Some(pid) = pid_opt {
            unsafe {
                let pm = crate::task::process_manager();
                (*pm).notify_thread_terminated(pid, id);
            }
            return true;
        }
        false
    }

    pub fn reap_terminated(&mut self) {
        for index in 0..MAX_THREADS {
            if index == IDLE_SLOT {
                continue;
            }

            if index == self.current {
                continue;
            }

            let should_reap = match &self.threads[index] {
                Some(thread) => thread.state() == ThreadState::Terminated,
                None => false,
            };

            if should_reap {
                let id = self.threads[index]
                    .as_ref()
                    .map(|thread| thread.id())
                    .unwrap_or(0);
                let pid = self.threads[index]
                    .as_ref()
                    .map(|t| t.process_id())
                    .unwrap_or(0);
                unsafe {
                    let pm = crate::task::process_manager();
                    (*pm).notify_thread_terminated(pid, id);
                }
                self.threads[index] = None;
                self.count = self.count.saturating_sub(1);

                crate::log_vfs_raw!("Scheduler: reaped thread=");
                crate::log_sched_raw!("{}", id);
                crate::log_sched_raw!(" slot=");
                crate::log_sched_raw!("{}", index as u64);
                crate::log_sched!("");
            }
        }
    }

    pub fn join(&mut self, id: ThreadID) {
        let current_id = match self.current_id() {
            Some(id) => id,
            None => return,
        };

        if current_id == id {
            return;
        }

        let target_exists = self
            .threads
            .iter()
            .flatten()
            .any(|thread| thread.id() == id);

        if !target_exists {
            return;
        }

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_waiting_for(Some(id));
            thread.set_state(ThreadState::Waiting);

            crate::log_vfs!("JOIN: current thread waiting");
        }

        loop {
            unsafe {
                core::arch::asm!("hlt");
            }

            let state = self.threads[self.current]
                .as_ref()
                .map(|thread| thread.state());

            if state != Some(ThreadState::Waiting) {
                break;
            }
        }
    }

    pub fn remove_thread_by_id(&mut self, id: ThreadID) -> bool {
        for slot in self.threads.iter_mut() {
            if let Some(thread) = slot {
                if thread.id() == id {
                    *slot = None;
                    self.count = self.count.saturating_sub(1);
                    return true;
                }
            }
        }
        false
    }

    pub fn create_user_thread(
        &mut self,
        cpu_id: usize,
        process_manager: &mut crate::task::process::ProcessManager,
        process_id: crate::task::process::ProcessID,
        user_rip: u64,
        user_rsp: u64,
    ) -> Option<ThreadID> {
        if self.count >= MAX_THREADS {
            crate::log_vfs_raw!("CREATE USER THREAD: scheduler full, reaping cpu=");
            crate::log_sched_raw!("{}", cpu_id as u64);
            crate::log_sched!("");

            self.reap_terminated();

            if self.count >= MAX_THREADS {
                crate::log_vfs_raw!("CREATE USER THREAD FAIL: scheduler still full cpu=");
                crate::log_sched_raw!("{}", cpu_id as u64);
                crate::log_sched_raw!(" count=");
                crate::log_sched_raw!("{}", self.count as u64);
                crate::log_sched_raw!(" max=");
                crate::log_sched_raw!("{}", MAX_THREADS as u64);
                crate::log_sched!("");
                return None;
            }
        }

        if process_manager.get(process_id).is_none() {
            crate::log_vfs_raw!("CREATE USER THREAD FAIL: process missing pid=");
            crate::log_sched_raw!("{}", process_id);
            crate::log_sched!("");
            return None;
        }

        let thread_id = process_manager.allocate_thread_id();

        crate::log_vfs_raw!("CREATE USER THREAD: allocated tid=");
        crate::log_sched_raw!("{}", thread_id);
        crate::log_smp_raw!(" cpu=");
        crate::log_sched_raw!("{}", cpu_id as u64);
        crate::log_sched_raw!(" pid=");
        crate::log_sched_raw!("{}", process_id);
        crate::log_sched!("");

        let thread = Thread::new_user(thread_id, cpu_id, process_id, user_rip, user_rsp);

        crate::log_vfs!("CREATE USER THREAD: Thread::new_user OK");

        if !self.add_thread(thread, cpu_id) {
            crate::log_vfs_raw!("CREATE USER THREAD FAIL: add_thread cpu=");
            crate::log_sched_raw!("{}", cpu_id as u64);
            crate::log_sched_raw!(" count=");
            crate::log_sched_raw!("{}", self.count as u64);
            crate::log_sched!("");
            return None;
        }

        crate::log_vfs_raw!("CREATE USER THREAD: scheduler add OK tid=");
        crate::log_sched_raw!("{}", thread_id);
        crate::log_sched!("");

        if !process_manager.add_thread(process_id, thread_id) {
            crate::log_vfs_raw!("CREATE USER THREAD: process registration FAILED tid=");
            crate::log_sched_raw!("{}", thread_id);
            crate::log_sched!("");
            self.remove_thread_by_id(thread_id);
            return None;
        }

        crate::log_vfs_raw!("CREATE USER THREAD: process registration OK tid=");
        crate::log_sched_raw!("{}", thread_id);
        crate::log_sched!("");

        let current_cpu = crate::arch::x86_64::smp::percpu::current_cpu_id();

        if cpu_id != current_cpu {
            crate::log_smp_raw!("CREATE USER THREAD: RESCHEDULE CPU=");
            crate::log_sched_raw!("{}", cpu_id as u64);
            crate::log_sched!("");

            crate::arch::x86_64::lapic::send_reschedule_ipi(cpu_id as u32);
        }

        Some(thread_id)
    }
}

extern "C" fn idle_thread() -> ! {
    loop {
        unsafe {
            core::arch::asm!("sti");
            core::arch::asm!("hlt");
        }
    }
}
