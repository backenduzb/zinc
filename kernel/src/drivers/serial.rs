use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86_64::io::{inb, outb};

const COM1: u16 = 0x3F8;

static SERIAL_LOCK: AtomicBool = AtomicBool::new(false);

fn lock() -> u64 {
    let flags: u64;

    unsafe {
        core::arch::asm!("pushfq", "pop {}", out(reg) flags, options(nomem, preserves_flags));
        core::arch::asm!("cli", options(nomem, nostack, preserves_flags));
    }

    while SERIAL_LOCK
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }

    flags
}

fn unlock(saved: u64) {
    SERIAL_LOCK.store(false, Ordering::Release);

    if saved & (1 << 9) != 0 {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
        }
    }
}

fn emit(byte: u8) {
    unsafe {
        while inb(COM1 + 5) & 0x20 == 0 {
            core::hint::spin_loop();
        }

        outb(COM1, byte);
    }
}

pub fn init() {
    unsafe {
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x80);

        outb(COM1 + 0, 0x03);
        outb(COM1 + 1, 0x00);

        outb(COM1 + 3, 0x03);

        outb(COM1 + 2, 0xC7);
        outb(COM1 + 4, 0x0B);
    }
}

pub fn putc(byte: u8) {
    let saved = lock();
    emit(byte);
    unlock(saved);
}

pub fn write_bytes(bytes: &[u8]) {
    let saved = lock();

    for &byte in bytes {
        emit(byte);
    }

    unlock(saved);
}

pub fn print(text: &str) {
    write_bytes(text.as_bytes());
}

pub fn println(text: &str) {
    let saved = lock();

    for byte in text.bytes() {
        emit(byte);
    }

    emit(b'\n');
    unlock(saved);
}

pub fn print_hex_u32(value: u32) {
    let hex = b"0123456789ABCDEF";
    let saved = lock();

    for i in (0..8).rev() {
        let digit = ((value >> (i * 4)) & 0xF) as usize;
        emit(hex[digit]);
    }

    unlock(saved);
}

pub fn print_hex_u64(value: u64) {
    let hex = b"0123456789ABCDEF";
    let saved = lock();

    for i in (0..16).rev() {
        let digit = ((value >> (i * 4)) & 0xF) as usize;
        emit(hex[digit]);
    }

    unlock(saved);
}

pub fn print_u64(mut value: u64) {
    let saved = lock();

    if value == 0 {
        emit(b'0');
        unlock(saved);
        return;
    }

    let mut buffer = [0u8; 20];
    let mut index = 0;

    while value > 0 {
        buffer[index] = b'0' + (value % 10) as u8;
        value /= 10;
        index += 1;
    }

    while index > 0 {
        index -= 1;
        emit(buffer[index]);
    }

    unlock(saved);
}

pub fn print_char(c: char) {
    let mut buf = [0u8; 4];
    let bytes = c.encode_utf8(&mut buf);
    write_bytes(bytes.as_bytes());
}

use core::fmt;

pub fn print_fmt(args: fmt::Arguments<'_>) {
    use core::fmt::Write;

    let saved = lock();
    let mut writer = SerialWriter;
    let _ = writer.write_fmt(args);
    unlock(saved);
}

pub fn println_fmt(args: fmt::Arguments<'_>) {
    use core::fmt::Write;

    let saved = lock();
    let mut writer = SerialWriter;
    let _ = writer.write_fmt(args);
    emit(b'\n');
    unlock(saved);
}

struct SerialWriter;

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            emit(byte);
        }
        Ok(())
    }
}