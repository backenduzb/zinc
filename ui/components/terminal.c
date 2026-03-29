#include <fb/fb.h>
#include <stdint.h>
#include <utils/string.h>
#include <font/psf1.h>
#include <ui/layout.h>

uint32_t col = 15;
uint32_t row = 38;
char *term_pointer = "$root%zinc > ";
int GLYPH_HEIGHT = 16;

uint32_t write_pointer(uint32_t x, uint32_t y, uint32_t color) {
    char *ptr = term_pointer;
    uint32_t glyph_h = psf1_get_height();
    uint32_t glyph_w = GLYPH_WIDTH;

    while (*ptr) {
        draw_char(x, y, *ptr++, color);
        x += glyph_w;
    }
    return x;
}

void termwrite(char ch, uint32_t color) {
    uint32_t glyph_h = psf1_get_height();

    if (ch == '\n') {
        col = 15;
        row += glyph_h;
        col = write_pointer(col, row, color);
        return;
    }

    if (col + GLYPH_WIDTH > width) { 
        col = 15;
        row += glyph_h;
    }

    if (row + glyph_h > height) {
        row = 38;
        col = 15;
    }

    draw_char(col, row, ch, color);
    col += GLYPH_WIDTH;
}
