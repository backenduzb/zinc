#include <fb/fb.h>
#include <stdint.h>

uint32_t *framebuffer = 0;
uint32_t pitch = 0;
uint32_t width = 0;
uint32_t height = 0;

void draw_line(void) {
    uint32_t max_x = (width > 100) ? width : 100;
    uint32_t end_x = (max_x > 700) ? 700 : max_x;
    for (uint32_t x = 100; x < end_x; x++) {
        framebuffer[200 * (pitch / 4) + x] = 0x00FF0000;
    }
}
void draw_line_white(void) {
    uint32_t max_x = (width > 100) ? width : 100;
    uint32_t end_x = (max_x > 700) ? 700 : max_x;
    for (uint32_t x = 100; x < end_x; x++) {
        framebuffer[200 * (pitch / 4) + x] = 0x00FFFFFF;
    }
}
