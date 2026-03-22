#include <stdint.h>
#include <fb/fb.h>
#include <fb/drawer.h>

void fill_rect(uint32_t x, uint32_t y, uint32_t w, uint32_t h, uint32_t color) {
    if (w == 0) w = width;
    if (x >= width || y >= height) return;
    if (x + w > width)  w = width - x;
    if (y + h > height) h = height - y;
    
    for (uint32_t yy = 0; yy < h; yy++) {
        uint32_t *rowp = (uint32_t *)((uint8_t *)framebuffer + (y + yy ) * pitch);
        for (uint32_t xx = 0; xx < w; xx++) {
            rowp[x + xx] = color;
        }
    }
}