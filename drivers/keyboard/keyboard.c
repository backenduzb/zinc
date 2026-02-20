#include <kernel/io.h>
#include <kernel/keyboard/keyboard.h>
#include <kernel/keyboard/keyreader.h>
#include <kernel/pic.h>
#include <kernel/string/str.h>
#include <kernel/vga/vga.h>

#include <kernel/vga/cursor.h>
#include <stdint.h>

volatile uint8_t last_scancode = 0;
volatile uint8_t kbd_has_data = 0;

void keyboard_handler() {
    uint8_t sc = inb(0x60);

    char c = scancode_to_char(sc);

    if (c) {
        char buf[2] = {c, 0};
        vga_write(buf);
    }

    if (sc == 0x4D) vga_right();
    else if (sc == 0x4B) vga_left();
    else if (sc == 0x48) vga_top();
    else if (sc == 0x50) vga_bottom();
    else if (sc == 0x0E) vga_backspace();

    pic_send_eoi(1);
}