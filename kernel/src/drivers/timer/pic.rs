use crate::arch::x86_64::io::{inb, io_wait, outb};

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;

const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

pub fn init() {
    unsafe {
        let master_mask = inb(PIC1_DATA);
        let slave_mask = inb(PIC2_DATA);

        io_wait();
        outb(PIC1_COMMAND, 0x11);
        io_wait();
        outb(PIC2_COMMAND, 0x11);
        io_wait();

        outb(PIC1_DATA, 32);
        io_wait();
        outb(PIC2_DATA, 40);
        io_wait();

        outb(PIC1_DATA, 0x04);
        io_wait();
        outb(PIC2_DATA, 0x02);
        io_wait();

        outb(PIC1_DATA, 0x01);
        io_wait();
        outb(PIC2_DATA, 0x01);
        io_wait();

        let _ = master_mask;
        let _ = slave_mask;

        outb(PIC1_DATA, 0xFE);
        io_wait();
        outb(PIC2_DATA, 0xFF);
        io_wait();
    }

    crate::drivers::serial::println("PIC initialized");
}

pub fn end_of_interrupt() {
    unsafe {
        outb(PIC1_COMMAND, PIC_EOI);
    }
}