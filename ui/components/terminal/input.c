#include <fb/fb.h>
#include <font/psf1.h>

#include "terminal.h"

static void terminal_wrap_if_needed(uint32_t glyph_h) {
    if (terminal_state.col + GLYPH_WIDTH > (width - TERMINAL_PADDING_X)) {
        terminal_state.col = TERMINAL_PADDING_X;
        terminal_state.row += glyph_h;
    }

    if (terminal_state.row + glyph_h > (height - TERMINAL_BOTTOM_PADDING)) {
        terminal_state.row = TERMINAL_INPUT_RESET_ROW;
        terminal_state.col = TERMINAL_PADDING_X;
    }
}

void termwrite(char ch, uint32_t color) {
    uint32_t glyph_h;

    if (!terminal_state.opened) {
        if (ch != '\n') {
            return;
        }

        terminal_clear();
        terminal_state.opened = 1;
    }

    if (ch == 0) {
        return;
    }

    glyph_h = psf1_get_height();
    cursor_update(terminal_state.row + TERMINAL_CURSOR_OFFSET_Y, terminal_state.col + GLYPH_WIDTH, 0x00FFFFFF);

    if (ch == '\n') {
        terminal_state.input_buffer[terminal_state.input_length] = '\0';
        terminal_execute_command(terminal_state.input_buffer);
        terminal_state.col = TERMINAL_PADDING_X;
        terminal_state.row += glyph_h;
        terminal_state.input_length = 0;
        terminal_state.input_buffer[0] = '\0';
        terminal_wrap_if_needed(glyph_h);
        terminal_state.col = terminal_write_prompt(terminal_state.col, terminal_state.row, color);
        return;
    }

    if (terminal_state.input_length + 1u >= TERMINAL_INPUT_BUFFER_SIZE) {
        return;
    }

    terminal_wrap_if_needed(glyph_h);
    terminal_state.input_buffer[terminal_state.input_length++] = ch;
    terminal_state.input_buffer[terminal_state.input_length] = '\0';
    draw_char((int)terminal_state.col, (int)terminal_state.row, ch, color);
    terminal_state.col += GLYPH_WIDTH;
}
