use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use crate::arch::x86_64::io::{inb, outb};

const KEYBOARD_DATA: u16 = 0x60;
const KEYBOARD_STATUS: u16 = 0x64;

const CMD_DISABLE_P1: u8 = 0xAD;
const CMD_DISABLE_P2: u8 = 0xA7;
const CMD_READ_CONFIG: u8 = 0x20;
const CMD_WRITE_CONFIG: u8 = 0x60;
const CMD_SELF_TEST: u8 = 0xAA;
const CMD_TEST_P1: u8 = 0xAB;
const CMD_ENABLE_P1: u8 = 0xAE;

const KBD_RESET: u8 = 0xFF;
const KBD_ENABLE_SCAN: u8 = 0xF4;

const KBD_ACK: u8 = 0xFA;
const KBD_SELFTEST_OK: u8 = 0xAA;

const ST_OBF: u8 = 0x01;
const ST_IBF: u8 = 0x02;

const TIMEOUT: u32 = 200_000;

const BUFFER_SIZE: usize = 256;

#[derive(Clone, Copy)]
pub struct KeyEvent {
    pub scancode: u8,
    pub pressed: bool,
}

static INPUT_PENDING: AtomicBool = AtomicBool::new(false);
static HEAD: AtomicU8 = AtomicU8::new(0);
static TAIL: AtomicU8 = AtomicU8::new(0);

static mut BUFFER: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];

#[inline]
fn status() -> u8 {
    unsafe { inb(KEYBOARD_STATUS) }
}

fn wait_send() -> bool {
    for _ in 0..TIMEOUT {
        if status() & ST_IBF == 0 {
            return true;
        }

        core::hint::spin_loop();
    }

    false
}

fn wait_recv() -> bool {
    for _ in 0..TIMEOUT {
        if status() & ST_OBF != 0 {
            return true;
        }

        core::hint::spin_loop();
    }

    false
}

fn flush_output() {
    while status() & ST_OBF != 0 {
        unsafe {
            inb(KEYBOARD_DATA);
        }
    }
}

fn read_data() -> Option<u8> {
    if !wait_recv() {
        return None;
    }

    Some(unsafe { inb(KEYBOARD_DATA) })
}

fn write_ctl(command: u8) -> bool {
    if !wait_send() {
        return false;
    }

    unsafe {
        outb(KEYBOARD_STATUS, command);
    }

    true
}

fn write_dev(value: u8) -> bool {
    if !wait_send() {
        return false;
    }

    unsafe {
        outb(KEYBOARD_DATA, value);
    }

    true
}

fn keyboard_command(command: u8) -> Option<u8> {
    if !write_dev(command) {
        return None;
    }

    read_data()
}

fn push_scancode(scancode: u8) {
    let head = HEAD.load(Ordering::Relaxed);
    let next = head.wrapping_add(1);

    let tail = TAIL.load(Ordering::Acquire);

    if next == tail {
        return;
    }

    unsafe {
        BUFFER[head as usize] = scancode;
    }

    HEAD.store(next, Ordering::Release);
    INPUT_PENDING.store(true, Ordering::Release);
}

pub fn try_read() -> Option<KeyEvent> {
    let tail = TAIL.load(Ordering::Relaxed);
    let head = HEAD.load(Ordering::Acquire);

    if tail == head {
        return None;
    }

    let scancode = unsafe { BUFFER[tail as usize] };

    TAIL.store(tail.wrapping_add(1), Ordering::Release);

    Some(KeyEvent {
        scancode: scancode & 0x7F,
        pressed: scancode & 0x80 == 0,
    })
}

pub fn input_pending() -> bool {
    INPUT_PENDING.load(Ordering::Acquire)
}

pub fn clear_pending() {
    if HEAD.load(Ordering::Acquire) == TAIL.load(Ordering::Acquire) {
        INPUT_PENDING.store(true, Ordering::Release);
    }
}

pub fn init() {
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
    }

    write_ctl(CMD_DISABLE_P1);
    write_ctl(CMD_DISABLE_P2);

    flush_output();

    if write_ctl(CMD_READ_CONFIG) {
        if let Some(config) = read_data() {
            let config = (config | 0x01) & !(1 << 4);

            if write_ctl(CMD_WRITE_CONFIG) {
                write_dev(config);
            }
        }
    }

    if write_ctl(CMD_SELF_TEST) {
        let _ = read_data();
    }

    if write_ctl(CMD_TEST_P1) {
        let _ = read_data();
    }

    write_ctl(CMD_ENABLE_P1);

    flush_output();

    if let Some(KBD_ACK) = keyboard_command(KBD_RESET) {
        let _ = read_data();
    }

    let _ = keyboard_command(KBD_ENABLE_SCAN);

    flush_output();

    crate::drivers::timer::pic::unmask_irq(1);

    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn keyboard_handler(_frame: *mut crate::arch::x86_64::context::InterruptFrame) {
    if status() & ST_OBF != 0 {
        let scancode = unsafe { inb(KEYBOARD_DATA) };

        push_scancode(scancode);

        if let Some(tid) = crate::tty::keyboard_tid() {
            crate::task::wake_thread(tid);
        }
    }

    crate::drivers::timer::pic::end_of_interrupt();
}
