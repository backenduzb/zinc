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

        crate::drivers::serial::print("ENTERING RING3: slot=");
        crate::drivers::serial::print_u64(index as u64);
        crate::drivers::serial::print(" tid=");
        crate::drivers::serial::print_u64(thread.id());
        crate::drivers::serial::print(" RIP=0x");
        crate::drivers::serial::print_hex_u64(frame.rip);
        crate::drivers::serial::print(" CS=0x");
        crate::drivers::serial::print_hex_u64(frame.cs);
        crate::drivers::serial::print(" RSP=0x");
        crate::drivers::serial::print_hex_u64(frame.rsp);
        crate::drivers::serial::print(" SS=0x");
        crate::drivers::serial::print_hex_u64(frame.ss);
        crate::drivers::serial::println("");
        crate::drivers::serial::print("RING3 CTX: CR3=0x");
        crate::drivers::serial::print_hex_u64(cr3);
        crate::drivers::serial::print(" TSS.RSP0=0x");
        crate::drivers::serial::print_hex_u64(rsp0);
        crate::drivers::serial::println("");

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

        crate::drivers::serial::print("TSS AUDIT: TR=0x");
        crate::drivers::serial::print_hex_u64(tr_selector as u64);
        crate::drivers::serial::print(" GDTR base=0x");
        crate::drivers::serial::print_hex_u64(gdtr_base);
        crate::drivers::serial::print(" limit=");
        crate::drivers::serial::print_u64(gdtr_limit as u64);
        crate::drivers::serial::println("");

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

        crate::drivers::serial::print("TSS AUDIT: desc base=0x");
        crate::drivers::serial::print_hex_u64(base);
        crate::drivers::serial::print(" limit=");
        crate::drivers::serial::print_u64(limit);
        crate::drivers::serial::print(" P=");
        crate::drivers::serial::print_u64(present);
        crate::drivers::serial::print(" type=0x");
        crate::drivers::serial::print_hex_u64(desc_type);
        crate::drivers::serial::println("");

        let cpu_rsp0 = unsafe { core::ptr::read_volatile((base + 4) as *const u64) };

        crate::drivers::serial::print("TSS AUDIT: cpu-visible RSP0=0x");
        crate::drivers::serial::print_hex_u64(cpu_rsp0);
        crate::drivers::serial::print(" percpu RSP0=0x");
        crate::drivers::serial::print_hex_u64(rsp0);
        crate::drivers::serial::println("");

        {
            let cpu = crate::arch::x86_64::smp::percpu::current();
            let true_tss = unsafe { cpu.tss.get() as u64 };
            let percpu_base = cpu as *const _ as u64;

            crate::drivers::serial::print("TSS AUDIT: percpu=0x");
            crate::drivers::serial::print_hex_u64(percpu_base);
            crate::drivers::serial::print(" tss=0x");
            crate::drivers::serial::print_hex_u64(true_tss);
            crate::drivers::serial::print(" desc=0x");
            crate::drivers::serial::print_hex_u64(base);

            if base == true_tss && cpu_rsp0 == rsp0 {
                crate::drivers::serial::println(" MATCH");
            } else {
                crate::drivers::serial::println(" MISMATCH");
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
            crate::drivers::serial::println("TSS AUDIT: RSP0 page walk OK");
        } else {
            crate::drivers::serial::println("TSS AUDIT: RSP0 page walk FAILED");
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
            crate::drivers::serial::print("CR3 SWITCH: 0x");
            crate::drivers::serial::print_u64(current);
            crate::drivers::serial::print(" -> 0x");
            crate::drivers::serial::print_u64(target);
            crate::drivers::serial::println("");

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

                crate::drivers::serial::print("TSS.RSP0 configured: CPU=");
                crate::drivers::serial::print_u64(
                    crate::arch::x86_64::smp::percpu::current_cpu_id() as u64,
                );
                crate::drivers::serial::print(" RSP0=0x");
                crate::drivers::serial::print_hex_u64(stack_top);
                crate::drivers::serial::println("");
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

        crate::drivers::serial::print("Scheduler: starting thread slot ");
        crate::drivers::serial::print_u64(first as u64);
        crate::drivers::serial::println("");

        self.switch_address_space_for(first);

        self.started = true;

        if is_user {
            if let Some(reason) = self.validate_fresh_user(first) {
                crate::drivers::serial::print("RING3 REFUSED (start_first): ");
                crate::drivers::serial::println(reason);

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
                crate::drivers::serial::print("AP Scheduler tick: CPU=");
                crate::drivers::serial::print_u64(cpu as u64);
                crate::drivers::serial::print(" ticks=");
                crate::drivers::serial::print_u64(self.ticks);
                crate::drivers::serial::println("");
            }

            if let Some(thread) = self.threads[self.current].as_mut() {
                thread.set_interrupt_rsp(frame as u64);
            }

            let now = crate::arch::x86_64::lapic::ticks();

            for thread in self.threads.iter_mut() {
                if let Some(thread) = thread.as_mut() {
                    if thread.state() == ThreadState::Sleeping && thread.wake_at() <= now {
                        thread.set_state(ThreadState::Ready);
                        crate::drivers::serial::print("CPU=");
                        crate::drivers::serial::print_u64(cpu as u64);
                        crate::drivers::serial::print(" TID=");
                        crate::drivers::serial::print_u64(thread.id());
                        crate::drivers::serial::println(" WAKE");
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

            crate::drivers::serial::print("CPU=");
            crate::drivers::serial::print_u64(cpu as u64);
            crate::drivers::serial::print(" current=");
            crate::drivers::serial::print_u64(prev as u64);
            crate::drivers::serial::print(" -> next=");
            crate::drivers::serial::print_u64(next as u64);
            crate::drivers::serial::println("");
            if prev == IDLE_SLOT && next != IDLE_SLOT {
                crate::drivers::serial::print("CPU=");
                crate::drivers::serial::print_u64(cpu as u64);
                crate::drivers::serial::print(" IDLE -> TID=");
                crate::drivers::serial::print_u64(next_tid);
                crate::drivers::serial::println("");
            } else if prev != IDLE_SLOT && next == IDLE_SLOT {
                crate::drivers::serial::print("CPU=");
                crate::drivers::serial::print_u64(cpu as u64);
                crate::drivers::serial::print(" TID=");
                crate::drivers::serial::print_u64(prev_tid);
                crate::drivers::serial::println(" -> IDLE");
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
                        crate::drivers::serial::print("RING3 REFUSED: ");
                        crate::drivers::serial::println(reason);

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
        crate::drivers::serial::print("TIME=");
        crate::drivers::serial::print_u64(now);
        crate::drivers::serial::println("");

        for thread in self.threads.iter_mut() {
            if let Some(thread) = thread.as_mut() {
                if thread.state() == ThreadState::Sleeping {
                    crate::drivers::serial::print("SLEEP CHECK thread=");
                    crate::drivers::serial::print_u64(thread.id());

                    crate::drivers::serial::print(" wake=");
                    crate::drivers::serial::print_u64(thread.wake_at());

                    crate::drivers::serial::print(" now=");
                    crate::drivers::serial::print_u64(now);

                    if thread.wake_at() <= now {
                        thread.set_state(ThreadState::Ready);
                        crate::drivers::serial::println(" -> WAKE");
                    } else {
                        crate::drivers::serial::println(" -> WAIT");
                    }
                }
            }
        }

        crate::drivers::serial::print("SCHED tick CPU=");
        crate::drivers::serial::print_u64(crate::arch::x86_64::smp::percpu::current_cpu_id() as u64);
        crate::drivers::serial::print(" current=");
        crate::drivers::serial::print_u64(self.current as u64);
        crate::drivers::serial::print(" next=");

        match self.next() {
            Some(index) => crate::drivers::serial::print_u64(index as u64),
            None => crate::drivers::serial::print_u64(99),
        }

        crate::drivers::serial::println("");

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
            crate::drivers::serial::print("Scheduler: ");
            crate::drivers::serial::print_u64(self.switches);
            crate::drivers::serial::print(" switches, now on slot ");
            crate::drivers::serial::print_u64(next as u64);
            crate::drivers::serial::println("");
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
                    crate::drivers::serial::print("RING3 REFUSED: ");
                    crate::drivers::serial::println(reason);

                    if let Some(thread) = self.threads[next].as_mut() {
                        thread.set_state(ThreadState::Terminated);
                    }

                    return (frame as u64, 0);
                }

                self.log_user_entry(next);

                let rsp = self.threads[next].as_ref().unwrap().interrupt_rsp();

                crate::drivers::serial::print("FRESH USER SWITCH: slot=");
                crate::drivers::serial::print_u64(next as u64);
                crate::drivers::serial::print(" thread=");
                crate::drivers::serial::print_u64(
                    self.threads[next].as_ref().map(|t| t.id()).unwrap_or(0),
                );
                crate::drivers::serial::println(" via start_user_thread_context");

                return (rsp, Self::user_entry_trampoline());
            }

            let thread = self.threads[next].as_ref().unwrap();

            let stack = thread.stack_top();
            let entry = thread.entry();

            crate::drivers::serial::print("FRESH SWITCH: slot=");
            crate::drivers::serial::print_u64(next as u64);

            crate::drivers::serial::print(" thread=");
            crate::drivers::serial::print_u64(thread.id());

            crate::drivers::serial::print(" entry=");
            crate::drivers::serial::print_u64(entry);

            crate::drivers::serial::print(" stack=");
            crate::drivers::serial::print_u64(stack);

            crate::drivers::serial::println("");

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
                    crate::drivers::serial::print("RING3 REFUSED (syscall yield): ");
                    crate::drivers::serial::println(reason);

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
                    crate::drivers::serial::print("RING3 REFUSED (syscall exit): ");
                    crate::drivers::serial::println(reason);

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

        crate::drivers::serial::print("TIME=");
        crate::drivers::serial::print_u64(now);
        crate::drivers::serial::println("");

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_wake_at(now + ticks);
            thread.set_state(ThreadState::Sleeping);

            crate::drivers::serial::print("SLEEP thread=");
            crate::drivers::serial::print_u64(thread.id());
            crate::drivers::serial::print(" wake_at=");
            crate::drivers::serial::print_u64(now + ticks);
            crate::drivers::serial::println("");

            crate::drivers::serial::print("CPU=");
            crate::drivers::serial::print_u64(cpu as u64);
            crate::drivers::serial::print(" TID=");
            crate::drivers::serial::print_u64(thread.id());
            crate::drivers::serial::println(" SLEEP");
        }
    }

    pub fn exit_current(&mut self) {
        let (exiting_id, pid) = match self.threads[self.current].as_ref() {
            Some(thread) => (thread.id(), thread.process_id()),
            None => return,
        };

        if let Some(thread) = self.threads[self.current].as_mut() {
            thread.set_state(ThreadState::Terminated);

            serial::print("EXIT thread=");
            serial::print_u64(exiting_id);
            serial::println("");
        }

        unsafe {
            let pm = crate::task::process_manager();
            (*pm).notify_thread_terminated(pid, exiting_id);
        }

        for index in 0..MAX_THREADS {
            if index == self.current {
                continue;
            }

            if let Some(thread) = self.threads[index].as_mut() {
                if thread.state() == ThreadState::Waiting
                    && thread.waiting_for() == Some(exiting_id)
                {
                    thread.set_waiting_for(None);
                    thread.set_state(ThreadState::Ready);

                    serial::print("JOIN: waking thread=");
                    serial::print_u64(thread.id());
                    serial::println("");
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

                    crate::drivers::serial::print("KILL thread=");
                    crate::drivers::serial::print_u64(id);
                    crate::drivers::serial::println("");

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

                crate::drivers::serial::print("Scheduler: reaped thread=");
                crate::drivers::serial::print_u64(id);
                crate::drivers::serial::print(" slot=");
                crate::drivers::serial::print_u64(index as u64);
                crate::drivers::serial::println("");
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

            serial::println("JOIN: current thread waiting");
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
            return None;
        }

        if process_manager.get(process_id).is_none() {
            return None;
        }

        let thread_id = process_manager.allocate_thread_id();

        let thread = Thread::new_user(thread_id, cpu_id, process_id, user_rip, user_rsp);

        if !self.add_thread(thread, cpu_id) {
            return None;
        }

        if !process_manager.add_thread(process_id, thread_id) {
            self.remove_thread_by_id(thread_id);
            return None;
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
