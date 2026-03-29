#include <fb/fb.h>
#include <stdint.h>
#include <utils/string.h>
#include <font/psf1.h>
#include <ui/layout.h>

uint32_t col = 0;
uint32_t row = 0;
char *term_pointer = "!zinc > ";

void termwrite(char *text, uint32_t color) {
    uint32_t glyph_h = psf1_get_height();
    uint32_t start_x = ui_get_content_origin_x();
    uint32_t start_y = ui_get_content_origin_y();
    uint32_t max_x = width;
    
    if (glyph_h == 0) return;
    if (width <= start_x + GLYPH_WIDTH) return;

    max_x = width - start_x - GLYPH_WIDTH;

    if (col < start_x) {
        col = start_x;
    }
    if (row < start_y) {
        row = start_y;
    }

    while (*text) {
        if (*text == '\n') {
            col = start_x;
            row += glyph_h;
            text++;
            continue;
        }

        if (col > max_x) {
            col = start_x;
            row += glyph_h;
        }
        if (row + glyph_h > height) {
            row = start_y;
        }

        draw_char(col, row, *text++, color);
        col += GLYPH_WIDTH;
    }
}
