#include <fb/fb.h>
#include <stdint.h>
#include <utils/string.h>
#include <font/psf1.h>
#include <ui/layout.h>

uint32_t col = 15;
uint32_t row = 38;
char *term_pointer = "$root%zinc > ";
int GLYPH_HEIGHT = 16;
int opened = 0;

char key_counter[256];
int key_index = 0;

static void clear_term() {
    for (uint32_t r = 0; r < height; r++) {
        for (uint32_t c = 0; c < width; c++) {
            framebuffer[r * pitch + c] = 0x00000000; 
        }
    }
    col = 15;
    row = 38;
}

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
    if (!opened && ch == '\n') {
        opened = 1; 
    }

    if (!opened) return;

    uint32_t glyph_h = psf1_get_height();

    if (ch == '\n') {
        key_counter[key_index] = '\0';    
        if (strcmp(key_counter, "clear") == 0) {
            clear_term();
        }

        col = 15;
        row += glyph_h;
        key_index = 0; 
        key_counter[0] = '\0';
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

    key_counter[key_index++] = ch;
    key_counter[key_index] = '\0'; // har doim terminate qilish

    draw_char(col, row, ch, color);
    col += GLYPH_WIDTH;
}
