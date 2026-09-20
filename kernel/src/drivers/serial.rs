use crate::arch::x86_64::io::{inb, outb};

const COM1: u16 = 0x3F8;

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
    unsafe {
        while inb(COM1 + 5) & 0x20 == 0 {
            core::hint::spin_loop();
        }

        outb(COM1, byte);
    }
}


pub fn print(text: &str) {
    for byte in text.bytes() {
        putc(byte);
    }
}

pub fn println(text: &str) {
    print(text);
    putc(b'\n');
}

pub fn print_hex_u32(mut value: u32) {
    let hex = b"0123456789ABCDEF";

    for i in (0..8).rev() {
        let digit = ((value >> (i * 4)) & 0xF) as usize;
        putc(hex[digit]);
    }
}

pub fn print_hex_u64(value: u64) {
    let hex = b"0123456789ABCDEF";
    for i in (0..16).rev() {
        let digit = ((value >> (i * 4)) & 0xF) as usize;
        putc(hex[digit]);
    }
}

pub fn print_u64(mut value: u64) {
    if value == 0 {
        putc(b'0');
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
        putc(buffer[index]);
    }
}

pub fn print_char(c: char) {
    let mut buf = [0u8; 4];
    let bytes = c.encode_utf8(&mut buf);
    for &byte in bytes.as_bytes() {
        putc(byte);
    }
}