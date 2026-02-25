#include <zinc/interrupts/keyboard.h>
#include <zinc/interrupts/pic.h>
#include <zinc/interrupts/idt.h>
#include <zinc/io.h>
#include <zinc/video/framebuffer.h>
#include <stdint.h>

static const char scancode_set1[128] = {
    0,  27, '1','2','3','4','5','6','7','8','9','0','-','=', '\b',
    '\t','q','w','e','r','t','y','u','i','o','p','[',']','\n',
    0, 'a','s','d','f','g','h','j','k','l',';','\'','`',
    0, '\\','z','x','c','v','b','n','m',',','.','/',
    0, 0, 0, ' ', 0,
};

static size_t kbd_col = 0;
static size_t kbd_row = 0;
static volatile uint64_t kbd_irq_cnt = 0;

static void handle_scancode(uint8_t sc) {
    if (sc & 0x80) {
        return;
    }

    char c = 0;
    if (sc < sizeof(scancode_set1)) {
        c = scancode_set1[sc];
    }
    char buff[2];
    buff[0] = c;
    buff[1] = '\0';
    if (c) {
        draw_string(buff, 0xFFFFFF);
    }
}

static int wait_input_clear(void) {
    for (int i = 0; i < 100000; i++) {
        if ((inb(0x64) & 0x02) == 0) return 0;
    }
    return -1;
}

static int wait_output_full(void) {
    for (int i = 0; i < 100000; i++) {
        if (inb(0x64) & 0x01) return 0;
    }
    return -1;
}

static void kbd_write_cmd(uint8_t cmd) {
    if (wait_input_clear() == 0) outb(0x64, cmd);
}

static void kbd_write_data(uint8_t data) {
    if (wait_input_clear() == 0) outb(0x60, data);
}

static uint8_t kbd_read_data(void) {
    if (wait_output_full() == 0) return inb(0x60);
    return 0;
}

void keyboard_init(void) {
    kbd_write_cmd(0xAD); 
    kbd_write_cmd(0xA7); 
    
    while (inb(0x64) & 0x01) {
        (void)inb(0x60);
    }
    kbd_write_cmd(0xAA);
    uint8_t resp = kbd_read_data();
    
    if (resp != 0x55) {
        draw_string("NO 8042", 0xFF0000);
    }
    kbd_write_data(0xF4);
    uint8_t ack = kbd_read_data();
    
    if (ack != 0xFA) {
        draw_string("NO ACK", 0xFF0000);
    }
    kbd_write_cmd(0x20);
    uint8_t cmd = kbd_read_data();
    cmd |= 0x01;  
    cmd |= 0x40;  
    cmd &= ~0x10; 
    cmd &= ~0x20; 

    kbd_write_cmd(0x60);
    kbd_write_data(cmd);

    kbd_write_cmd(0xAE);

    kbd_write_data(0xF4);
    (void)kbd_read_data(); 

    while (inb(0x64) & 0x01) {
        (void)inb(0x60);
    }
}

__attribute__((interrupt))
void irq1_handler(struct interrupt_frame *frame) {
    (void)frame;
    uint8_t sc = inb(0x60);
    kbd_irq_cnt++;
    handle_scancode(sc);

    pic_eoi(1);
}
