#include <commands/commands.h>
#include <kernel/vga/vga.h>
#include <kernel/vga/colors.h>
#include <kernel/string/str.h>
#include <commands/power.h>

#define REGISTER_COMMAND(name, func) {name, func}

void clear_cmd(int argc, char **argv){
    (void)argc;
    (void)argv;
    vga_clear();
}

void write_wrong_command(char *command){
    vga_set_color(VGA_LIGHT_GREEN, VGA_BLACK);
    vga_write("\n bash: ");
    vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);
    vga_write("command");
    vga_set_color(VGA_LIGHT_RED, VGA_BLACK);
    vga_write(" NOT ");
    vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);
    vga_write("not found: '");
    vga_write(command);
    vga_write("'\n");
}

void help(int argc, char **argv){
    vga_write("\nZinc OS help for help clear for clear screen. \n");
    vga_set_color(VGA_LIGHT_GREEN, VGA_BLACK);
    vga_write("You can laught with kernel, you need enter hahaha.");
    vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);
    vga_write("\n");
    if(argc > 1){
        vga_write("Argument: ");
        vga_write(argv[1]);
        vga_write("\n");
    }
}

void kernel_lol(int argc, char **argv){
    (void)argc;
        (void)argv;
    vga_set_color(VGA_MANAGENTA, VGA_BLACK);
    vga_write("\nVAXAXAXAXAXAXAXA!");
    vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);
    vga_write("\n");
}

Command commands[] = {
    REGISTER_COMMAND("help", help),
    REGISTER_COMMAND("clear", clear_cmd),
    REGISTER_COMMAND("hahaha", kernel_lol),
    REGISTER_COMMAND("reboot", reboot),
    REGISTER_COMMAND("mdown", shutdown),
};

static int command_count = sizeof(commands) / sizeof(Command);

int parse_input(char *input, char **argv){
    int argc = 0;

    while(*input){
        while(*input == ' ') input++;
        if(*input == 0) break;

        argv[argc++] = input;

        while(*input && *input != ' ') input++;
        if(*input){
            *input = 0;
            input++;
        }
    }

    return argc;
}

Command* find_command(const char *input) {

    char *argv[10];
    int argc = parse_input(input, argv);

    if(argc == 0)
        return 0;

    for(int i = 0; i < command_count; i++){
        if(strcmp(argv[0], commands[i].name) == 0){
            commands[i].func(argc, argv);
            return &commands[i];
        }
    }

    write_wrong_command(argv[0]);
    return 0;
}
