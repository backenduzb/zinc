#include <commands/commands.h>
#include <kernel/vga/vga.h>
#include <kernel/vga/colors.h>
#include <kernel/string/str.h>

#define REGISTER_COMMAND(name, func) {name, func}

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

void help(){
    vga_write("\nZinc OS help for help clear for clear screen. \n");
    vga_set_color(VGA_LIGHT_GREEN, VGA_BLACK);
    vga_write("You can laught with kernel, you need enter hahaha.");
    vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);
    vga_write("\n");
}

void kernel_lol(){
    vga_set_color(VGA_MANAGENTA, VGA_BLACK);
    vga_write("\nVAXAXAXAXAXAXAXA!");
    vga_set_color(VGA_LIGHT_GREY, VGA_BLACK);
    vga_write("\n");
}

Command commands[] = {
    REGISTER_COMMAND("help", help),
    REGISTER_COMMAND("clear", vga_clear),
    REGISTER_COMMAND("hahaha", kernel_lol),
};

static int command_count = sizeof(commands) / sizeof(Command);

Command* find_command(const char *input) {
    for (int i = 0; i < command_count; i++) {       
        if (strcmp(input, commands[i].name) == 0){
            return &commands[i];
        }
    }
    
    write_wrong_command(input);
    return 0;
}
