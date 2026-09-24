use crate::arch::x86_64::io::{inb, io_wait, outb};

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;

const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

const MASTER_MASK: u8 = 0xFC;
const SLAVE_MASK: u8 = 0xFF;

pub fn init() {
    unsafe {
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

        outb(PIC1_DATA, MASTER_MASK);
        io_wait();

        outb(PIC2_DATA, SLAVE_MASK);
        io_wait();
    }
} 

pub fn unmask_irq(irq: u8) {
    unsafe {
        if irq < 8 {
            let mask = inb(PIC1_DATA) & !(1 << irq);
            outb(PIC1_DATA, mask);
            io_wait();
        } else if irq < 16 {
            let mask = inb(PIC2_DATA) & !(1 << (irq - 8));
            outb(PIC2_DATA, mask);
            io_wait();
        }
    }
}

pub fn end_of_interrupt() {
    unsafe {
        outb(PIC1_COMMAND, PIC_EOI);
    }
}