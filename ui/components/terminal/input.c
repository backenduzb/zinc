#include <fb/fb.h>
#include <font/psf1.h>
#include <stdint.h>

#include "terminal.h"

static int is_hex(char c) {
    return (c >= '0' && c <= '9') ||
           (c >= 'a' && c <= 'f') ||
           (c >= 'A' && c <= 'F');
}

static uint32_t parse_hex(const char *s) {
    uint32_t val = 0;

    for (int i = 0; i < 8; i++) {
        val <<= 4;

        if (s[i] >= '0' && s[i] <= '9')
            val |= (s[i] - '0');
        else if (s[i] >= 'a' && s[i] <= 'f')
            val |= (s[i] - 'a' + 10);
        else if (s[i] >= 'A' && s[i] <= 'F')
            val |= (s[i] - 'A' + 10);
    }

    return val;
}

void writewc(uint32_t x, uint32_t y, const char *str, uint32_t default_color) {
    uint32_t color = default_color;
    uint32_t start_x = x;

    for (int i = 0; str[i]; i++) {

        if (str[i] == '\n') {
            y += psf1_get_height();
            x = start_x;
            continue;
        }
        if (str[i] == '\\' && str[i+1] == '\\' && str[i+2] == '\\') {

            int j = i + 3;

            if (str[j] == '0' && str[j+1] == 'x') {
                j += 2;

                int ok = 1;
                for (int k = 0; k < 8; k++) {
                    if (!is_hex(str[j + k])) {
                        ok = 0;
                        break;
                    }
                }

                if (ok &&
                    str[j+8] == '\\' &&
                    str[j+9] == '\\' &&
                    str[j+10] == '\\') {

                    color = parse_hex(&str[j]);
                    i = j + 10; 
                    continue;
                }
            }
        }
        draw_char(x, y, str[i], color);
        x += 8;
    }
}

static void clear_char(uint32_t row, uint32_t col) {
    for (uint32_t r = 0; r < 16; r++) {
        for (uint32_t c = 0; c < 8; c++) {
            framebuffer[(row + r) * width + (col + c)] = 0x000000;
        }
    }
}

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

    if (ch == '\n') {
        terminal_state.input_buffer[terminal_state.input_length] = '\0';
        int command_founded = terminal_execute_command(terminal_state.input_buffer);
        terminal_state.row += glyph_h;
        terminal_state.col = TERMINAL_PADDING_X;
        if (!command_founded && terminal_state.input_length != 0) {
            writewc(
                terminal_state.col,
                terminal_state.row + glyph_h,
                "bash: command \\\\\\0x00FF0000\\\\\\ NOT \\\\\\0x00FFFFFF\\\\\\ found!",
                0x00AAAAAA
            );
            terminal_state.row += glyph_h * 2;
        }
        terminal_state.input_length = 0;
        terminal_state.input_buffer[0] = '\0';
        terminal_wrap_if_needed(glyph_h);
        terminal_state.col = terminal_write_prompt(terminal_state.col, terminal_state.row, color);
        return;
    }

    if (ch == '\b') {
        if (terminal_state.input_length > 0) {
            terminal_state.input_length--;
            terminal_state.input_buffer[terminal_state.input_length] = '\0';
    
            terminal_state.col -= GLYPH_WIDTH;
            clear_char(terminal_state.row, terminal_state.col);
        }
    
        cursor_update(
            terminal_state.row + TERMINAL_CURSOR_OFFSET_Y,
            terminal_state.col,
            0x00FFFFFF
        );
        return;
    }

    if (terminal_state.input_length + 1u >= TERMINAL_INPUT_BUFFER_SIZE) {
        return;
    }

    cursor_update(
        terminal_state.row + TERMINAL_CURSOR_OFFSET_Y,
        terminal_state.col + GLYPH_WIDTH,
        0x00FFFFFF
    );
    terminal_wrap_if_needed(glyph_h);
    terminal_state.input_buffer[terminal_state.input_length++] = ch;
    terminal_state.input_buffer[terminal_state.input_length] = '\0';
    draw_char((int)terminal_state.col, (int)terminal_state.row, ch, color);
    terminal_state.col += GLYPH_WIDTH;
}
