#include <stdint.h>
#include <stddef.h>
#include <zinc/video/framebuffer.h>
#include <zinc/time/btime.h>
#include <zinc/interrupts/interrupts.h>
#include <zinc/interrupts/keyboard.h>


void kernel_main(void) {
    framebuffer_init();
    uint32_t white = 0xFFFFFF;
    uint32_t cyan = 0x00FFFF;
    uint32_t gray = 0x8D8D8D;
    draw_string_center("Welcome to ZINC OS!", cyan);
    
    draw_string("\n/root%zinc > ", white);
    write_time();
    interrupts_init();
    interrupts_enable();
    while (1) {
        __asm__ volatile ("hlt");
    }
}
