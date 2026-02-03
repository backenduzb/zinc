#include <kernel/vga/vga.h>
#include <kernel/vga/colors.h>

void kernel_main(void){
    vga_clear();
    vga_set_color(VGA_CYAN, VGA_BLACK);
    vga_write("Zinc OS\n");
    vga_write("VGA s\n");
}
