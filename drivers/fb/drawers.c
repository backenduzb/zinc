#include <stdint.h>
#include <fb/drawer.h>
#include <font/psf1.h>
#include <timer/pit.h>
#include <utils/string.h>

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

static uint32_t font_height(void) {
    return psf1_get_height();
}

void write(const char *text, uint32_t color) {
    uint32_t glyph_h = font_height();
    if (glyph_h == 0) {
        return;
    }

    while (*text) {
        if (*text == '\n') {
            row += glyph_h;
            col = 0;
            text++;
            continue;
        }
        if (col + GLYPH_WIDTH >= width) {
            col = 0;
            row += glyph_h;
        }
        if (row + glyph_h >= height) return;
        draw_char((int)col, (int)row, *text++, color);
        col += GLYPH_WIDTH;
    }
}

void write_center(const char *text, uint32_t color) {
    uint32_t glyph_h = font_height();
    if (glyph_h == 0) {
        return;
    }

    uint32_t len = strlen(text);
    uint32_t text_width = len * GLYPH_WIDTH;
    uint32_t text_height = glyph_h;

    uint32_t x = (width - text_width) / 2;
    uint32_t y = (height - text_height) / 2;

    while (*text) {
        draw_char((int)x, (int)y, *text++, color);
        x += GLYPH_WIDTH;
    }
}

void write_center_with_duration(const char *text, uint32_t color, uint32_t duration) {
    uint32_t glyph_h = font_height();
    if (glyph_h == 0) {
        return;
    }

    uint32_t len = strlen(text);
    uint32_t text_width = len * GLYPH_WIDTH;
    uint32_t text_height = glyph_h;

    uint32_t x = (width - text_width) / 2;
    uint32_t y = (height - text_height) / 2;

    while (*text) {
        draw_char((int)x, (int)y, *text++, color);
        x += GLYPH_WIDTH;
        sleep(duration);
    }
}
void draw_resolution(void) {
    char text[16];
    uitoa(width, text);
    write(text, 0x00FFFFFF);

    uitoa(height, text);
    write("x", 0x00FFFFFF);
    write(text, 0x00FFFFFF);
}

void draw_background(){ 
    for (int y = 0; y < height; y++) {
        for (int x = 0; x < width; x++) {
            // framebuffer[y * (pitch/4) +x] = img[y * width + x];
        }
    }
}