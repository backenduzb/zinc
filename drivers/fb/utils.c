#include <fb/fb.h>
#include <stdint.h>

uint32_t *framebuffer = 0;
uint32_t pitch = 0;
uint32_t width = 0;
uint32_t height = 0;

void draw_line(void) {
    for (int x = 100; x < 700; x++) {
        framebuffer[200 * (pitch / 4) + x] = 0x00FF0000;
    }
}
void draw_line_white(void) {
    for (int x = 100; x < 700; x++) {
        framebuffer[200 * (pitch / 4) + x] = 0x00FFFFFF;
    }
}