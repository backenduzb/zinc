#include <power.h>
#include <utils/string.h>
#include <string.h>
#include "terminal.h"

typedef void (*terminal_command_handler_t)(int argc, char **argv);

typedef struct {
    const char *name;
    terminal_command_handler_t handler;
} terminal_command_t;

static void command_clear(int argc, char **argv) {
    terminal_clear();
}

static void command_shutdown(int argc, char **argv) {
    shutdown();
}

static void command_reboot(int argc, char **argv) {
    reboot();
}

static const terminal_command_t terminal_commands[] = {
    { "clear", command_clear },
    { "mdown", command_shutdown },
    { "reboot", command_reboot },
};

int terminal_execute_command(char *command) {
    char *argv[10];
    int argc = 0;

    char *buffer = strtok(command, " ");
    while (buffer != NULL && argc < 10) {
        argv[argc++] = buffer;
        buffer = strtok(NULL, " ");
    }

    if (argc == 0) return 0;

    for (uint32_t i = 0; i < (sizeof(terminal_commands) / sizeof(terminal_commands[0])); ++i) {
        if (strcmp(argv[0], terminal_commands[i].name) == 0) {
            terminal_commands[i].handler(argc, argv);
            return 1;
        }
    }

    return 0;
}
