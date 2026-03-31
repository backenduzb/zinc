#include <power.h>
#include <utils/string.h>

#include "terminal.h"

typedef void (*terminal_command_handler_t)(void);

typedef struct {
    const char *name;
    terminal_command_handler_t handler;
} terminal_command_t;

static void command_clear(void) {
    terminal_clear();
}

static void command_shutdown(void) {
    shutdown();
}

static void command_reboot(void) {
    reboot();
}

static const terminal_command_t terminal_commands[] = {
    { "clear", command_clear },
    { "mdown", command_shutdown },
    { "reboot", command_reboot },
};

int terminal_execute_command(const char *command) {
    for (uint32_t i = 0; i < (sizeof(terminal_commands) / sizeof(terminal_commands[0])); ++i) {
        if (strcmp(command, terminal_commands[i].name) == 0) {
            terminal_commands[i].handler();
            return 1;
        }
    }
    return 0;
}