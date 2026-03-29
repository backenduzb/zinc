#include <fb/fb.h>
#include <stdint.h>
#include <utils/string.h>
#include <font/psf1.h>

uint32_t col = 0;
uint32_t row = 38;
char *term_pointer = "!zinc > ";

void termwrite(char *text, uint32_t color) {
    uint32_t glyph_h = psf1_get_height();
    
    if (glyph_h == 0) return;

    while (*text) {

        if (col >= (width - GLYPH_WIDTH)) {
            col = 0;
            row += glyph_h;
        }

        draw_char(col, row, *text++, color);
        col += GLYPH_WIDTH;
    }
}
