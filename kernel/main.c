#include <stdint.h>
#include <stddef.h>
#include <zinc/video/framebuffer.h>

void kernel_main(void) {
    framebuffer_init();
    uint32_t white = 0xFFFFFF;
    uint32_t gray = 0x8D8D8D;
    draw_string_scaled(850, 530, "Welcome to", white, 2.5);
    draw_string_scaled(1020, 530, "ZINC !", gray, 2.5); 
    while (1) {
        __asm__ volatile ("hlt");
    }
}
