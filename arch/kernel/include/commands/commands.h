#ifndef COMMANDS_H
#define COMMANDS_H

typedef void (*command_func)();

typedef struct {
    const char *name;
    command_func func;
} Command;

Command* find_command(const char *input);

#endif