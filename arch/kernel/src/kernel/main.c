#include <kernel/vga/vga.h>
#include <kernel/vga/colors.h>

void kernel_main(void){
    vga_clear();
    vga_set_color(VGA_WHITE, VGA_BLACK);
    vga_write("#######  ####   ###   ##    ###### \n");
    vga_write("    ##    ##    ## #  ##    ##     \n");
    vga_write("  ##      ##    ##  # ##    ##     \n");
    vga_write("#######  ####   ##   ###    ###### \n");
}
