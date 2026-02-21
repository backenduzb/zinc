#include <kernel/io.h>
#include <kernel/keyboard/keyboard.h>
#include <kernel/keyboard/keyreader.h>
#include <kernel/pic.h>
#include <kernel/string/str.h>
#include <kernel/vga/vga.h>
#include <commands/commands.h>
#include <kernel/vga/cursor.h>
#include <stdint.h>

volatile uint8_t last_scancode = 0;
volatile uint8_t kbd_has_data = 0;

int key_idx = 0;
char key_counter[256];

void keyboard_handler() {
    uint8_t sc = inb(0x60);
    
    
    
    if (sc == 0x1C) {
        key_counter[key_idx] = '\0';   

        Command* cmd = find_command(key_counter);
        if (cmd) {
            cmd->func();
        }
        key_idx = 0;
        vga_write("\n/root%zinc > ");
        pic_send_eoi(1);
        return;
    }

    if (sc == 0x0E) {
        if (key_idx > 0) {
            key_idx--;
            vga_backspace();
        }
        pic_send_eoi(1);
        return;
    }
    

    char c = scancode_to_char(sc);

    if (c && key_idx < 255) {
        key_counter[key_idx++] = c;

        char buf[2] = {c, 0};
        vga_write(buf);
    }

    if (sc == 0x4D) vga_right();
    else if (sc == 0x4B) vga_left();
    else if (sc == 0x48) vga_top();
    else if (sc == 0x50) vga_bottom();

    pic_send_eoi(1);
}