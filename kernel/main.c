#include <stdint.h>
#include <stddef.h>
#include <zinc/video/framebuffer.h>

void kernel_main(void) {
    framebuffer_init();
    uint32_t white = 0xFFFFFF;
    uint32_t gray = 0x8D8D8D;
    draw_string("Welcome to ", white);
    draw_string("ZINC !", gray); 
    draw_string("                                           bakdsvbkkafkkj                                 bakdsvbkkafkkj                                 bakdsvbkkafkkj                                 bakdsvbkkafkkj                                 bakdsvbkkafkkj                                  bakdsvbkkafkkj                                 bakdsvbkkafkkj                                 bakdsvbkkafkkj", gray); 
    while (1) {
        __asm__ volatile ("hlt");
    }
}
