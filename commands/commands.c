#include <commands/commands.h>
#include <kernel/vga/vga.h>
#include <kernel/string/str.h>

#define REGISTER_COMMAND(name, func) {name, func}

void help(){
    vga_write("Zinc OS help for help clear for clear screen.");
}

Command commands[] = {
    REGISTER_COMMAND("help", help),
    REGISTER_COMMAND("clear", vga_clear),
};

static int command_count = sizeof(commands) / sizeof(Command);

Command* find_command(const char *input) {
    for (int i = 0; i < command_count; i++) {       
        if (strcmp(input, commands[i].name) == 0){
            return &commands[i];
        }
    }

    vga_write(input);
    vga_write(" Command not found!\n");
    return 0;
}
