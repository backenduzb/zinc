#include <stdint.h>
#include <fb/drawer.h>
#include <utils/string.h>
#include <font/font8x8.h>
#include <timer/pit.h>

uint32_t *framebuffer = 0;
uint32_t pitch = 0;
uint32_t width = 0;
uint32_t height = 0;
uint32_t row = 0;
uint32_t col = 0;

void draw_line(void) {
    uint32_t max_x = (width > 100) ? width : 100;
    uint32_t end_x = (max_x > 700) ? 700 : max_x;
    for (uint32_t x = 100; x < end_x; x++) {
        framebuffer[200 * (pitch / 4) + x] = 0x00FF0000;
    }
}

void draw_screen_border(uint32_t color) {
    for (uint32_t x = 0; x < width; x++) {
        framebuffer[1 * (pitch / 4) + x] = color;
    }
    for (uint32_t x = 0; x < width; x++) {
        framebuffer[(height-1) * (pitch / 4) + x] = color;
    }
}

void draw_line_white(void) {
    uint32_t max_x = (width > 100) ? width : 100;
    uint32_t end_x = (max_x > 700) ? 700 : max_x;
    for (uint32_t x = 100; x < end_x; x++) {
        framebuffer[200 * (pitch / 4) + x] = 0x00FFFFFF;
    }
}

static void draw_glyph(uint32_t x, uint32_t y, const glyph8x8_t *glyph, uint32_t color) {
    for (int d_row = 0; d_row < 8; d_row ++) {
        uint8_t bits = (*glyph)[d_row];
        for (int d_col = 0; d_col < 8; d_col ++) {
            if (bits & (1 << (7 - d_col))) {
                uint32_t px = x + d_col;
                uint32_t py = y + d_row;
                framebuffer[py * (pitch / 4) + px] = color;
            }
        }
    }
}

static const glyph8x8_t *lookup_char(char c) {
    if (c >= '0' && c <= '9') return (const glyph8x8_t *)&font8x8_digits[c - '0'];
    if (c >= 'a' && c <= 'z') return (const glyph8x8_t *)&font8x8_lowercase[c - 'a'];
    if (c >= 'A' && c <= 'Z') return (const glyph8x8_t *)&font8x8_uppercase[c - 'A'];
    return font8x8_symbol_for(c);
}

void write(const char *text, uint32_t color) {
    while (*text) {
        if (*text == '\n') {
            row += 8;
            col = 0;
            text++;
            continue;
        }
        if (col + 8 >= width) {
            col = 0;
            row += 8;
        }
        if (row + 8 >= height) return;
        const glyph8x8_t *glyph = lookup_char(*text++);
        if (glyph) {
            draw_glyph(col, row, glyph, color);
        }
        col += 8;
    }
}

void write_center(const char *text, uint32_t color) {
    uint32_t len = strlen(text);
    uint32_t text_width = len * 8;

    uint32_t x = (width - text_width) / 2;
    uint32_t y = (height - 8) / 2;

    while (*text) {
        const glyph8x8_t *glyph = lookup_char(*text++);
        if (glyph) {
            draw_glyph(x, y, glyph, color);
        }
        x += 8;
    }
}