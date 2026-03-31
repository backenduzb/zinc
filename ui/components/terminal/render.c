#include <fb/fb.h>
#include <font/psf1.h>
#include <time/time.h>

#include "terminal.h"

void terminal_clear(void) {
    for (uint32_t r = TERMINAL_CLEAR_TOP; r < height; ++r) {
        for (uint32_t c = TERMINAL_PADDING_X; c < (width - TERMINAL_PADDING_X); ++c) {
            framebuffer[r * width + c] = 0x00000000;
        }
    }

    terminal_state.col = TERMINAL_PADDING_X;
    terminal_state.row = TERMINAL_CLEAR_TOP;
    terminal_state.input_length = 0;
    terminal_state.input_buffer[0] = '\0';
    terminal_state.cursor_initialized = 0;
    terminal_state.cursor_last_toggle = 0;
    terminal_state.cursor_on = 1;
}

uint32_t terminal_write_prompt(uint32_t x, uint32_t y, uint32_t color) {
    const char *prompt = terminal_prompt;
    uint32_t glyph_w = GLYPH_WIDTH;
    uint32_t clock_pos = width - (glyph_w * 10u);
    char time_buffer[20];

    while (*prompt) {
        draw_char((int)x, (int)y, *prompt++, color);
        x += glyph_w;
    }

    get_time(time_buffer);
    for (char *time_text = time_buffer; *time_text; ++time_text) {
        draw_char((int)clock_pos, (int)y, *time_text, 0x0000FF00);
        clock_pos += glyph_w;
    }

    cursor_update(y + TERMINAL_CURSOR_OFFSET_Y, x, 0x00FFFFFF);
    return x;
}
