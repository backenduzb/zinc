#include <fb/fb.h>
#include <timer/pit.h>

#include "terminal.h"

static void cursor_paint(uint32_t row, uint32_t col, uint32_t *buffer, uint32_t color, uint8_t save_background) {
    for (uint32_t h = 0; h < TERMINAL_CURSOR_HEIGHT; ++h) {
        for (uint32_t w = 0; w < TERMINAL_CURSOR_WIDTH; ++w) {
            uint32_t index = (row + h) * width + (col + w);
            if (save_background) {
                buffer[h * TERMINAL_CURSOR_WIDTH + w] = framebuffer[index];
            }
            framebuffer[index] = color;
        }
    }
}

static void cursor_restore(uint32_t row, uint32_t col, const uint32_t *buffer) {
    for (uint32_t h = 0; h < TERMINAL_CURSOR_HEIGHT; ++h) {
        for (uint32_t w = 0; w < TERMINAL_CURSOR_WIDTH; ++w) {
            framebuffer[(row + h) * width + (col + w)] = buffer[h * TERMINAL_CURSOR_WIDTH + w];
        }
    }
}

void cursor_refresh(void) {
    uint32_t cursor_row;
    uint32_t cursor_col;

    if (!terminal_state.opened || !terminal_state.cursor_initialized) {
        return;
    }

    if ((ticks - terminal_state.cursor_last_toggle) < 700u) {
        return;
    }

    terminal_state.cursor_last_toggle = ticks;
    terminal_state.cursor_on = (uint8_t)!terminal_state.cursor_on;
    cursor_row = terminal_state.row + TERMINAL_CURSOR_OFFSET_Y;
    cursor_col = terminal_state.col;

    if (terminal_state.cursor_on) {
        cursor_paint(cursor_row, cursor_col, terminal_state.cursor_backup, 0x00FFFFFF, 0);
        return;
    }

    cursor_restore(cursor_row, cursor_col, terminal_state.cursor_backup);
}

void cursor_update(uint32_t row, uint32_t col, uint32_t color) {
    if (terminal_state.cursor_initialized) {
        cursor_restore(terminal_state.cursor_row_prev, terminal_state.cursor_col_prev, terminal_state.cursor_backup);
    }

    cursor_paint(row, col, terminal_state.cursor_backup, color, 1);
    terminal_state.cursor_row_prev = row;
    terminal_state.cursor_col_prev = col;
    terminal_state.cursor_initialized = 1;
    terminal_state.cursor_on = 1;
    terminal_state.cursor_last_toggle = ticks;
}
