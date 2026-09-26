use core::sync::atomic::{AtomicBool, Ordering};

use crate::drivers::input::keyboard::{self, KeyEvent};

const INPUT_BUFFER_SIZE: usize = 256;

struct InputBuffer {
    buffer: [u8; INPUT_BUFFER_SIZE],
    len: usize,
    ready: bool,
}

impl InputBuffer {
    const fn new() -> Self {
        Self {
            buffer: [0; INPUT_BUFFER_SIZE],
            len: 0,
            ready: false,
        }
    }

    fn push(&mut self, byte: u8) {
        if self.len >= INPUT_BUFFER_SIZE - 1 {
            return;
        }

        self.buffer[self.len] = byte;
        self.len += 1;
    }

    fn backspace(&mut self) {
        if self.len > 0 {
            self.len -= 1;
        }
    }

    fn finish_line(&mut self) {
        if self.len < INPUT_BUFFER_SIZE {
            self.buffer[self.len] = b'\n';
            self.len += 1;
        }

        self.ready = true;
    }

    fn take(&mut self, out: &mut [u8]) -> usize {
        if !self.ready {
            return 0;
        }

        let count = core::cmp::min(self.len, out.len());

        out[..count].copy_from_slice(&self.buffer[..count]);

        self.len = 0;
        self.ready = false;

        count
    }
}

struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    fn lock(&self) {
        while self
            .locked
            .compare_exchange(
                false,
                true,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

static INPUT_LOCK: SpinLock = SpinLock::new();
static mut INPUT: InputBuffer = InputBuffer::new();

fn with_input<F, R>(f: F) -> R
where
    F: FnOnce(&mut InputBuffer) -> R,
{
    INPUT_LOCK.lock();

    let result = unsafe {
        f(&mut *core::ptr::addr_of_mut!(INPUT))
    };

    INPUT_LOCK.unlock();

    result
}

pub fn process_pending() {
    while let Some(event) = keyboard::try_read() {
        handle_key(event);
    }

    keyboard::clear_pending();
}

fn handle_key(event: KeyEvent) {
    if !event.pressed {
        return;
    }

    match event.scancode {
        0x0E => {
            with_input(|input| {
                input.backspace();
            });

            crate::drivers::serial::putc(8);
            crate::drivers::serial::putc(b' ');
            crate::drivers::serial::putc(8);
        }

        0x1C => {
            with_input(|input| {
                input.finish_line();
            });

            crate::drivers::serial::putc(b'\n');
            crate::drivers::serial::print("zinc > ");
        }

        scancode => {
            if let Some(ch) = scancode_to_ascii(scancode) {
                with_input(|input| {
                    input.push(ch);
                });

                crate::drivers::serial::putc(ch);
            }
        }
    }
}

pub fn read(out: &mut [u8]) -> usize {
    with_input(|input| input.take(out))
}

fn scancode_to_ascii(scancode: u8) -> Option<u8> {
    Some(match scancode {
        0x02 => b'1',
        0x03 => b'2',
        0x04 => b'3',
        0x05 => b'4',
        0x06 => b'5',
        0x07 => b'6',
        0x08 => b'7',
        0x09 => b'8',
        0x0A => b'9',
        0x0B => b'0',

        0x10 => b'q',
        0x11 => b'w',
        0x12 => b'e',
        0x13 => b'r',
        0x14 => b't',
        0x15 => b'y',
        0x16 => b'u',
        0x17 => b'i',
        0x18 => b'o',
        0x19 => b'p',

        0x1E => b'a',
        0x1F => b's',
        0x20 => b'd',
        0x21 => b'f',
        0x22 => b'g',
        0x23 => b'h',
        0x24 => b'j',
        0x25 => b'k',
        0x26 => b'l',

        0x2C => b'z',
        0x2D => b'x',
        0x2E => b'c',
        0x2F => b'v',
        0x30 => b'b',
        0x31 => b'n',
        0x32 => b'm',

        0x39 => b' ',

        _ => return None,
    })
}