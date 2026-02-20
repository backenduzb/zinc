#include <kernel/io.h>
#include <kernel/keyboard/keyboard.h>
#include <kernel/vga/vga.h>
#include <kernel/pic.h>
#include <stdint.h>
#include <kernel/vga/cursor.h>

volatile uint8_t last_scancode = 0;
volatile uint8_t kbd_has_data = 0;

void keyboard_handler(){
    uint8_t sc = inb(0x60);

    if(sc == 0xE0 || (sc & 0x80)){
        pic_send_eoi(1);
        return;
    }
    
    last_scancode = sc;
    kbd_has_data = 1;

    pic_send_eoi(1);
}
