#include <fb/fb.h>
#include <font/psf1.h>
#include <stdint.h>

#include <utils/string.h>

#include "terminal.h"

static void terminal_wrap_output(uint32_t glyph_h) {
    if (terminal_state.col + GLYPH_WIDTH > (width - TERMINAL_PADDING_X)) {
        terminal_state.col = TERMINAL_PADDING_X;
        terminal_state.row += glyph_h;
    }

    if (terminal_state.row + glyph_h > (height - TERMINAL_BOTTOM_PADDING)) {
        terminal_state.row = TERMINAL_INPUT_RESET_ROW;
        terminal_state.col = TERMINAL_PADDING_X;
    }
}

static void terminal_write_raw_char(char ch, uint32_t color) {
    uint32_t glyph_h = psf1_get_height();

    if (glyph_h == 0 || ch == '\0') {
        return;
    }

    if (ch == '\n') {
        terminal_state.row += glyph_h;
        terminal_state.col = TERMINAL_PADDING_X;
        terminal_wrap_output(glyph_h);
        cursor_update(
            terminal_state.row + TERMINAL_CURSOR_OFFSET_Y,
            terminal_state.col,
            0x00FFFFFF
        );
        return;
    }

    if (ch == '\r') {
        terminal_state.col = TERMINAL_PADDING_X;
        terminal_wrap_output(glyph_h);
        cursor_update(
            terminal_state.row + TERMINAL_CURSOR_OFFSET_Y,
            terminal_state.col,
            0x00FFFFFF
        );
        return;
    }

    terminal_wrap_output(glyph_h);
    draw_char((int)terminal_state.col, (int)terminal_state.row, ch, color);
    terminal_state.col += GLYPH_WIDTH;

    cursor_update(
        terminal_state.row + TERMINAL_CURSOR_OFFSET_Y,
        terminal_state.col,
        0x00FFFFFF
    );
}

void terminal_writestring_char(char ch) {
    terminal_write_raw_char(ch, 0x00FFFFFF);
}

void terminal_writestring(const char *str) {
    if (str == 0) {
        return;
    }

    while (*str) {
        terminal_writestring_char(*str++);
    }
}

void terminal_writeint(uint32_t num) {
    char buffer[11];

    uitoa(num, buffer);
    terminal_writestring(buffer);
}

void terminal_writehex(uint32_t num, int digits) {
    char buffer[9];
    static const char hex_digits[] = "0123456789ABCDEF";

    if (digits < 1) {
        digits = 1;
    } else if (digits > 8) {
        digits = 8;
    }

    for (int i = digits - 1; i >= 0; --i) {
        buffer[i] = hex_digits[num & 0x0Fu];
        num >>= 4;
    }
    buffer[digits] = '\0';

    terminal_writestring(buffer);
}
