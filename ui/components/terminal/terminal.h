#pragma once

#include <stdint.h>

#define TERMINAL_PADDING_X 15u
#define TERMINAL_CLEAR_TOP 30u
#define TERMINAL_INPUT_RESET_ROW 38u
#define TERMINAL_BOTTOM_PADDING 10u
#define TERMINAL_CURSOR_OFFSET_Y 10u
#define TERMINAL_CURSOR_HEIGHT 3u
#define TERMINAL_CURSOR_WIDTH 8u
#define TERMINAL_INPUT_BUFFER_SIZE 256u

typedef struct {
    uint32_t col;
    uint32_t row;
    uint32_t cursor_row_prev;
    uint32_t cursor_col_prev;
    uint32_t cursor_backup[TERMINAL_CURSOR_HEIGHT * TERMINAL_CURSOR_WIDTH];
    uint32_t input_length;
    uint64_t cursor_last_toggle;
    uint8_t cursor_on;
    uint8_t cursor_initialized;
    int opened;
    char input_buffer[TERMINAL_INPUT_BUFFER_SIZE];
} terminal_state_t;

extern terminal_state_t terminal_state;
extern const char *terminal_prompt;

void terminal_clear(void);
uint32_t terminal_write_prompt(uint32_t x, uint32_t y, uint32_t color);
void terminal_execute_command(const char *command);
void cursor_update(uint32_t row, uint32_t col, uint32_t color);
