#include <fb/fb.h>
#include <stdint.h>
#include <utils/string.h>
#include <font/psf1.h>
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

void center_label(uint32_t y, char *text, uint32_t color) {
    uint32_t glyph_h = psf1_get_height();
    if (glyph_h == 0) {
        return;
    }
    
    uint32_t text_len = strlen(text);
    uint32_t text_width = text_len * GLYPH_WIDTH;
    uint32_t text_height = glyph_h;
    uint32_t new_y = y - GLYPH_WIDTH;
    uint32_t x = (width - text_width) / 2;
    
    while (*text) {
        draw_char((int)x, (int)new_y, *text++, color);
        x += GLYPH_WIDTH;
    }
}