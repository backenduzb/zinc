#include "terminal.h"

terminal_state_t terminal_state = {
    .col = TERMINAL_PADDING_X,
    .row = TERMINAL_INPUT_RESET_ROW,
    .cursor_on = 1,
};

const char *terminal_prompt = "$root%zinc > ";
